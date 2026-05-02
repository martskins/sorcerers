use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State, TemporaryEffect},
};

#[derive(Debug, Clone)]
pub struct SeaRaider {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SeaRaider {
    pub const NAME: &'static str = "Sea Raider";
    pub const DESCRIPTION: &'static str = "Whenever Sea Raider attacks and kills an enemy, its controller discards their topmost spell. You may cast that spell once this turn, ignoring threshold.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SeaRaider {
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

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let self_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::DamageDealt {
                    source: Some(CardQuery::from_id(self_id)),
                    target: None,
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, damaged_id: &uuid::Uuid, effect: &Effect| {
                        let damaged_id = *damaged_id;
                        Box::pin(async move {
                            let Effect::TakeDamage { from, .. } = effect else {
                                return Ok(vec![]);
                            };
                            if from != &self_id {
                                return Ok(vec![]);
                            }

                            let killed_enemy = state.effects.iter().any(|queued| {
                                matches!(queued.as_ref(), Effect::KillMinion { card_id, killer_id }
                                    if card_id == &damaged_id && killer_id == &self_id)
                            });
                            if !killed_enemy {
                                return Ok(vec![]);
                            }

                            let attacked_with_raider = state.effect_log.iter().rev().any(|logged| {
                                matches!(logged.effect.as_ref(), Effect::Attack { attacker_id, .. }
                                    if attacker_id == &self_id)
                            });
                            if !attacked_with_raider {
                                return Ok(vec![]);
                            }

                            let sea_raider = state.get_card(&self_id);
                            let controller = sea_raider.get_controller_id(state);
                            if state.get_card(&damaged_id).get_controller_id(state) == controller {
                                return Ok(vec![]);
                            }

                            let Some(&spell_id) = state.get_player_deck(&controller)?.peek_spell()
                            else {
                                return Ok(vec![]);
                            };

                            let expires_on_effect = EffectQuery::OneOf(vec![
                                EffectQuery::TurnEnd {
                                    player_id: Some(controller),
                                },
                                EffectQuery::PlayCard {
                                    card: CardQuery::from_id(spell_id),
                                },
                            ]);

                            Ok(vec![
                                Effect::DiscardCard {
                                    player_id: controller,
                                    card_id: spell_id,
                                },
                                Effect::AddTemporaryEffect {
                                    effect: TemporaryEffect::MakePlayable {
                                        affected_cards: CardQuery::from_id(spell_id)
                                            .including_not_in_play(),
                                        expires_on_effect: expires_on_effect.clone(),
                                        by_player: controller,
                                    },
                                },
                                Effect::AddTemporaryEffect {
                                    effect: TemporaryEffect::IgnoreCostThresholds {
                                        affected_cards: CardQuery::from_id(spell_id)
                                            .including_not_in_play(),
                                        expires_on_effect,
                                        for_player: controller,
                                    },
                                },
                            ])
                        })
                            as Pin<
                                Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                            >
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SeaRaider::NAME, |owner_id: PlayerId| {
    Box::new(SeaRaider::new(owner_id))
});
