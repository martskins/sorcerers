use std::{pin::Pin, sync::Arc};

use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct GiantShark {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GiantShark {
    pub const NAME: &'static str = "Giant Shark";
    pub const DESCRIPTION: &'static str = "Submerge, Waterbound\r \r Whenever another unit enters or moves between sites in this body of water, Giant Shark moves to that unit to fight it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Submerge, Ability::Waterbound],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GiantShark {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let shark_id = self.get_id().clone();
        let Some(body_of_water) = state.get_body_of_water_at(self.get_zone()) else {
            return Ok(vec![]);
        };

        Ok(vec![ContinuousEffect::AddTriggeredEffect {
            trigger_on_effect: EffectQuery::MoveCard {
                card: CardQuery::new().units(),
            },
            on_effect: Arc::new(
                move |state: &State, card_id: &uuid::Uuid, effect: &Effect| {
                    let player_id = state.get_card(card_id).get_controller_id(state);
                    let body_of_water = body_of_water.clone();
                    Box::pin(async move {
                        match effect {
                            Effect::MoveCard { to, .. } => {
                                let moved_to =
                                    to.resolve(&player_id, state).await.unwrap_or_default();
                                if !body_of_water.contains(&moved_to) {
                                    return Ok(vec![]);
                                }

                                Ok(vec![Effect::Attack {
                                    attacker_id: shark_id.clone(),
                                    defender_id: card_id.clone(),
                                }])
                            }
                            _ => Ok(vec![]),
                        }
                    })
                },
            ),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (GiantShark::NAME, |owner_id: PlayerId| {
        Box::new(GiantShark::new(owner_id))
    });
