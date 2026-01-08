use crate::{
    card::{AvatarBase, Card, CardBase, Edition, Plane, Rarity, Rubble, UnitBase, Zone, place_rubble},
    effect::Effect,
    game::{AvatarAction, CardAction, Element, PlayerId, Thresholds, force_sync, pick_card, pick_zone},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub enum GeomancerAbility {
    PlaySite,
    DrawSite,
    ReplaceRubble,
}

#[async_trait::async_trait]
impl CardAction for GeomancerAbility {
    fn get_name(&self) -> &str {
        match self {
            GeomancerAbility::PlaySite => "Play Site",
            GeomancerAbility::DrawSite => "Draw Site",
            GeomancerAbility::ReplaceRubble => "Replace Rubble",
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
                    .map(|c| c.get_id().clone())
                    .collect();
                let prompt = "Pick a site to play";
                let picked_card_id = pick_card(player_id, &cards, state, prompt).await?;
                let picked_card = state.get_card(&picked_card_id);
                let zones = picked_card.get_valid_play_zones(state)?;
                let prompt = "Pick a zone to play the site";
                let zone = pick_zone(player_id, &zones, state, prompt).await?;
                let mut effects: Vec<Effect> = vec![
                    Effect::play_card(player_id, &picked_card_id, &zone).into(),
                    Effect::tap_card(card_id).into(),
                ];

                let mut snapshot = state.snapshot();
                for effect in &effects {
                    effect.apply(&mut snapshot).await?;
                }
                snapshot.apply_effects_without_log().await?;
                force_sync(player_id, &snapshot).await?;

                let picked_site = state.get_card(&picked_card_id);
                if picked_site
                    .get_site()
                    .ok_or(anyhow::anyhow!("Not a site"))?
                    .provides(&Element::Earth)?
                    .unwrap_or(0)
                    > 0
                {
                    let card = state.get_card(card_id);
                    let zones = card
                        .get_zone()
                        .get_adjacent()
                        .iter()
                        .filter(|z| z.get_site(&snapshot).is_none())
                        .cloned()
                        .collect::<Vec<Zone>>();
                    let picked_zone =
                        pick_zone(player_id, &zones, state, "Geomancer: Pick a void to fill with a rubble").await?;
                    effects.extend(place_rubble(card.get_controller_id(), &picked_zone));
                }

                Ok(effects)
            }
            GeomancerAbility::DrawSite => Ok(AvatarAction::DrawSite.on_select(card_id, player_id, state).await?),
            GeomancerAbility::ReplaceRubble => {
                let card = state.get_card(card_id);
                let adjacent_zones = card.get_zone().get_adjacent();
                let cards = state
                    .cards
                    .iter()
                    .filter(|c| c.get_name() == Rubble::NAME)
                    .filter(|c| adjacent_zones.contains(c.get_zone()))
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<uuid::Uuid>>();
                let picked_rubble = pick_card(
                    card.get_controller_id(),
                    &cards,
                    state,
                    "Geomancer: Pick a rubble to replace with a site",
                )
                .await?;

                let rubble = state.get_card(&picked_rubble);
                let deck = state.decks.get(card.get_controller_id()).unwrap();
                let site_id = deck.sites.last();

                match site_id {
                    Some(site_id) => Ok(vec![
                        Effect::BuryCard {
                            card_id: rubble.get_id().clone(),
                            from: rubble.get_zone().clone(),
                        },
                        Effect::DrawSite {
                            player_id: card.get_controller_id().clone(),
                            count: 1,
                        },
                        Effect::MoveCard {
                            player_id: card.get_controller_id().clone(),
                            card_id: site_id.clone(),
                            from: Zone::Atlasbook,
                            to: ZoneQuery::Specific {
                                id: uuid::Uuid::new_v4(),
                                zone: rubble.get_zone().clone(),
                            },
                            tap: false,
                            plane: Plane::Surface,
                            through_path: None,
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
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Geomancer {
    pub const NAME: &'static str = "Geomancer";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            avatar_base: AvatarBase {},
        }
    }
}

impl Card for Geomancer {
    fn get_name(&self) -> &str {
        Self::NAME
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Geomancer::NAME, |owner_id: PlayerId| Box::new(Geomancer::new(owner_id)));
