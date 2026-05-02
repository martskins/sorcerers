use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, yes_or_no},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct ScourgeZombies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ScourgeZombies {
    pub const NAME: &'static str = "Scourge Zombies";
    pub const DESCRIPTION: &'static str = "Whenever an allied Mortal dies on land, you may summon Scourge Zombies from your cemetery to its location, tapped.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ScourgeZombies {
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
                    source: None,
                    target: Some(CardQuery::new().minions().minion_type(&MinionType::Mortal)),
                },
                expires_on_effect: None,
                on_effect: Arc::new(
                    move |state: &State, damaged_id: &uuid::Uuid, effect: &Effect| {
                        let damaged_id = *damaged_id;
                        Box::pin(async move {
                            let Effect::TakeDamage { from, .. } = effect else {
                                return Ok(vec![]);
                            };

                            let self_card = state.get_card(&self_id);
                            if *self_card.get_zone() != Zone::Cemetery {
                                return Ok(vec![]);
                            }

                            let self_controller = self_card.get_controller_id(state);
                            let damaged_card = state.get_card(&damaged_id);
                            if damaged_card.get_controller_id(state) != self_controller {
                                return Ok(vec![]);
                            }
                            if damaged_card.get_region(state) != &Region::Surface {
                                return Ok(vec![]);
                            }

                            let died_here = damaged_card.get_zone().clone();
                            if !died_here.is_in_play() {
                                return Ok(vec![]);
                            }

                            let died = state.effects.iter().any(|queued| {
                                matches!(queued.as_ref(), Effect::KillMinion { card_id, .. }
                                    if card_id == &damaged_id)
                            });
                            if !died {
                                return Ok(vec![]);
                            }

                            let attacker = state.get_card(from);
                            let has_lethal_target =
                                damaged_card.has_ability(state, &Ability::LethalTarget);
                            let will_die = attacker.has_ability(state, &Ability::Lethal)
                                || has_lethal_target
                                || damaged_card.get_unit_base().is_some_and(|ub| {
                                    ub.damage >= damaged_card.get_toughness(state).unwrap_or(0)
                                });
                            if !will_die {
                                return Ok(vec![]);
                            }

                            if !yes_or_no(
                                &self_controller,
                                state,
                                "Summon Scourge Zombies to the fallen Mortal's location tapped?",
                            )
                            .await?
                            {
                                return Ok(vec![]);
                            }

                            Ok(vec![
                                Effect::SummonCard {
                                    player_id: self_controller,
                                    card_id: self_id,
                                    zone: died_here,
                                },
                                Effect::TapCard { card_id: self_id },
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScourgeZombies::NAME, |owner_id: PlayerId| {
        Box::new(ScourgeZombies::new(owner_id))
    });
