use std::sync::Arc;

use crate::{
    card::{Ability, Card, CardBase, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, pick_card},
    query::EffectQuery,
    state::{DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct Blaze {
    card_base: CardBase,
}

impl Blaze {
    pub const NAME: &'static str = "Blaze";
    pub const DESCRIPTION: &'static str = "This turn, give an ally Movement +2, it can't be intercepted, and it leaves a trail of fire at departed locations. When it stops, each unit along the trail takes 2 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
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

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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
                        card: picked_card.into(),
                    },
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                    on_effect: Arc::new(|_state: &State, card_id: &uuid::Uuid, effect: &Effect| {
                        Box::pin(async move {
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

                                    Ok(effects)
                                }
                                _ => unreachable!(),
                            }
                        })
                    }),
                    multitrigger: false,
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Blaze::NAME, |owner_id: PlayerId| {
        Box::new(Blaze::new(owner_id))
    });
