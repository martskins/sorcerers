use crate::{
    card::{
        AvatarBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Rubble,
        UnitBase, Zone,
    },
    effect::{Effect, TokenType},
    game::{ActivatedAbility, AvatarAction, Element, PlayerId, pick_card, pick_zone},
    query::ZoneQuery,
    state::{CardQuery, State},
};

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
        card_id: &uuid::Uuid,
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
                    .cards_named(Rubble::NAME)
                    .adjacent_to(geomancer.get_zone())
                    .all(state)
                    .is_empty())
            }
        }
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        match self {
            GeomancerAbility::PlaySite => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone() == &Zone::Hand)
                    .filter(|c| c.get_owner_id() == player_id)
                    .map(|c| *c.get_id())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let zones = picked_card.get_valid_play_zones(state)?;
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                let mut effects: Vec<Effect> = vec![
                    Effect::PlayCard {
                        player_id: *player_id,
                        card_id: picked_card_id,
                        zone: zone.clone().into(),
                    },
                    Effect::TapCard { card_id: *card_id },
                ];

                let picked_site = state.get_card(&picked_card_id);
                let is_earth_site = picked_site
                    .get_resource_provider()
                    .ok_or(anyhow::anyhow!("Not a site"))?
                    .provided_affinity(state)?
                    .element(&Element::Earth)
                    > 0;
                if is_earth_site {
                    let card = state.get_card(card_id);
                    let zones = card
                        .get_zone()
                        .get_adjacent()
                        .iter()
                        .filter(|z| z.get_site(state).is_none())
                        .filter(|z| z != &&zone)
                        .cloned()
                        .collect::<Vec<Zone>>();
                    if !zones.is_empty() {
                        let picked_zone = pick_zone(
                            player_id,
                            &zones,
                            state,
                            false,
                            "Geomancer: Pick a void to fill with a rubble",
                        )
                        .await?;
                        effects.push(Effect::SummonToken {
                            player_id: card.get_controller_id(state),
                            token_type: TokenType::Rubble,
                            zone: picked_zone.clone(),
                        });
                    }
                }

                Ok(effects)
            }
            GeomancerAbility::DrawSite => Ok(AvatarAction::DrawSite
                .on_select(card_id, player_id, state)
                .await?),
            GeomancerAbility::ReplaceRubble => {
                let card = state.get_card(card_id);
                let adjacent_zones = card.get_zone().get_adjacent();
                let cards = state
                    .cards
                    .iter()
                    .filter(|c| c.get_name() == Rubble::NAME)
                    .filter(|c| adjacent_zones.contains(c.get_zone()))
                    .map(|c| *c.get_id())
                    .collect::<Vec<uuid::Uuid>>();
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
                        Effect::MoveCard {
                            player_id: card.get_controller_id(state),
                            card_id: *site_id,
                            from: Zone::Atlasbook,
                            to: ZoneQuery::from_zone(rubble.get_zone().clone()),
                            tap: false,
                            region: Region::Surface,
                            through_path: None,
                        },
                        Effect::TapCard {
                            card_id: *rubble.get_id(),
                        },
                    ]),
                    None => Ok(vec![]),
                }
            }
        }
    }
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
                region: Region::Surface,
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(GeomancerAbility::ReplaceRubble)])
    }

    fn base_avatar_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![
            Box::new(GeomancerAbility::PlaySite),
            Box::new(GeomancerAbility::DrawSite),
            Box::new(GeomancerAbility::ReplaceRubble),
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Geomancer::NAME, |owner_id: PlayerId| {
    Box::new(Geomancer::new(owner_id))
});
