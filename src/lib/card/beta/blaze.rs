use std::sync::Arc;

use crate::{
    card::{Ability, Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, pick_card},
    query::{CardQuery, EffectQuery},
    state::{DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct Blaze {
    pub card_base: CardBase,
}

impl Blaze {
    pub const NAME: &'static str = "Blaze";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(3, "F"),
                region: Region::Surface,
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
impl Card for Blaze {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let units = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_controller_id(state) == self.get_controller_id(state))
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let prompt = "Blaze: Pick an ally";
        let picked_card = pick_card(self.get_controller_id(state), &units, state, prompt).await?;
        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: picked_card.clone(),
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Movement(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                },
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::MoveCard {
                        card: CardQuery::from_id(picked_card),
                    },
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                    on_effect: Arc::new(|_state: &State, card_id: &uuid::Uuid, effect: &Effect| -> Vec<Effect> {
                        match effect {
                            Effect::MoveCard {
                                player_id,
                                through_path,
                                ..
                            } => {
                                let mut effects = vec![];
                                if let Some(path) = through_path {
                                    for zone in path {
                                        if Some(zone) != path.last() {
                                            effects.push(Effect::DealDamageAllUnitsInZone {
                                                player_id: player_id.clone(),
                                                zone: zone.clone().into(),
                                                from: card_id.clone(),
                                                damage: 2,
                                            });
                                        }
                                    }
                                }

                                effects
                            }
                            _ => unreachable!(),
                        }
                    }),
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Blaze::NAME, |owner_id: PlayerId| Box::new(Blaze::new(owner_id)));
