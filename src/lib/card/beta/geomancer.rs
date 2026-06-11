use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum GeomancerAbility {
    PlaySite,
    DrawSite,
    ReplaceRubble,
}

#[async_trait::async_trait]
impl ActivatedAbility for GeomancerAbility {
    fn get_name(&self) -> String {
        match self {
            GeomancerAbility::PlaySite => "Play Site".to_string(),
            GeomancerAbility::DrawSite => "Draw Site".to_string(),
            GeomancerAbility::ReplaceRubble => "Replace Rubble".to_string(),
        }
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        match self {
            GeomancerAbility::PlaySite => Ok(true),
            GeomancerAbility::DrawSite => Ok(true),
            GeomancerAbility::ReplaceRubble => {
                let geomancer = state.get_card(card_id);
                Ok(!CardQuery::new()
                    .sites()
                    .named(Rubble::NAME.to_string())
                    .adjacent_to(geomancer.get_zone())
                    .all(state)
                    .is_empty())
            }
        }
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            GeomancerAbility::PlaySite => {
                let cards: Vec<CardId> = state
                    .cards
                    .values()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| *c.get_id())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let avatar_id = state.get_player_avatar_id(player_id)?;
                // we pass avatar_id as the caster just to comply with the required parameters, but
                // no caster_id is actually needed here, since sites don't need one.
                let zones = picked_card.get_valid_play_zones(state, player_id, &avatar_id)?;
                let prompt = "Pick a zone to play the site";
                let locations = crate::game::zones_to_locations(&zones);
                let zone = pick_location(player_id, &locations, state, false, prompt).await?;
                geomancer_play_site_effects(
                    card_id,
                    player_id,
                    state,
                    picked_card_id,
                    zone.get_square().unwrap(),
                )
                .await
            }
            GeomancerAbility::DrawSite => Ok(AvatarAction::DrawSite
                .on_select(card_id, player_id, state)
                .await?),
            GeomancerAbility::ReplaceRubble => {
                let card = state.get_card(card_id);
                let adjacent_zones = card.get_zone().get_adjacent();
                let cards = state
                    .cards
                    .values()
                    .filter(|c| c.get_name() == Rubble::NAME)
                    .filter(|c| adjacent_zones.contains(c.get_zone()))
                    .map(|c| *c.get_id())
                    .collect::<Vec<CardId>>();
                let picked_rubble = pick_card(
                    card.get_controller_id(state),
                    &cards,
                    state,
                    "Geomancer: Pick a rubble to replace with a site",
                )
                .await?;

                let rubble = state.get_card(&picked_rubble);
                let deck = state.decks.get(&card.get_controller_id(state)).unwrap();
                let site_id = deck.sites.last();

                match site_id {
                    Some(site_id) => Ok(vec![
                        Effect::BanishCard {
                            card_id: *rubble.get_id(),
                        },
                        Effect::SetCardZone {
                            card_id: *site_id,
                            zone: rubble.get_zone().clone(),
                        },
                        Effect::SetTapped {
                            card_id: *card_id,
                            tapped: true,
                        },
                    ]),
                    None => Ok(vec![]),
                }
            }
        }
    }
}

async fn geomancer_play_site_effects(
    geomancer_id: &PlayerId,
    player_id: &PlayerId,
    state: &State,
    site_id: PlayerId,
    square: u8,
) -> anyhow::Result<Vec<Effect>> {
    let mut effects = vec![
        Effect::PlayCard {
            player_id: *player_id,
            card_id: site_id,
            location: Location::Square(square, Region::Surface),
            spellcaster: *geomancer_id,
        },
        Effect::SetTapped {
            card_id: *geomancer_id,
            tapped: true,
        },
    ];

    let picked_site = state.get_card(&site_id);
    let is_earth_site = picked_site
        .get_resource_provider()
        .ok_or(anyhow::anyhow!("Not a site"))?
        .provided_affinity(state)?
        .element(&Element::Earth)
        > 0;
    if is_earth_site {
        let geomancer = state.get_card(geomancer_id);
        let zones = geomancer
            .get_zone()
            .get_adjacent()
            .iter()
            .filter(|z| z.get_site(state).is_none())
            .filter(|z| z.get_square().unwrap_or_default() != square)
            .cloned()
            .collect::<Vec<Zone>>();
        if !zones.is_empty() {
            let locations = crate::game::zones_to_locations(&zones);
            let picked_zone = pick_location(
                player_id,
                &locations,
                state,
                false,
                "Geomancer: Pick a void to fill with a rubble",
            )
            .await?;
            effects.push(Effect::SummonToken {
                player_id: geomancer.get_controller_id(state),
                token_type: TokenType::Rubble,
                location: picked_zone,
            });
        }
    }

    Ok(effects)
}

#[derive(Debug, Clone)]
pub struct Geomancer {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Geomancer {
    pub const NAME: &'static str = "Geomancer";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site. If you played an earth site, fill a void adjacent to you with Rubble.\r \r Tap → Replace an adjacent Rubble with the topmost site of your atlas.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Geomancer {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_avatar(&self) -> Option<&dyn Avatar> {
        Some(self)
    }

    fn base_get_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        let mut actions: Vec<Box<dyn ActivatedAbility>> =
            self.base_unit_activated_abilities(state)?;
        actions.extend(vec![
            Box::new(GeomancerAbility::PlaySite) as Box<dyn ActivatedAbility>,
            Box::new(GeomancerAbility::DrawSite) as Box<dyn ActivatedAbility>,
            Box::new(GeomancerAbility::ReplaceRubble) as Box<dyn ActivatedAbility>,
        ]);

        Ok(actions)
    }
}

#[async_trait::async_trait]
impl Avatar for Geomancer {
    fn get_play_site_ability(&self) -> Option<Box<dyn ActivatedAbility>> {
        Some(Box::new(GeomancerAbility::PlaySite))
    }

    async fn play_site_at_square(
        &self,
        state: &State,
        player_id: &PlayerId,
        site_id: &CardId,
        square: u8,
    ) -> anyhow::Result<Vec<Effect>> {
        geomancer_play_site_effects(self.get_id(), player_id, state, *site_id, square).await
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Geomancer::NAME, |owner_id: PlayerId| {
    Box::new(Geomancer::new(owner_id))
});
