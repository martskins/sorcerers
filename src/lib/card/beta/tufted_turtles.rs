use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct TuftedTurtles {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TuftedTurtles {
    pub const NAME: &'static str = "Tufted Turtles";
    pub const DESCRIPTION: &'static str =
        "The first time Tufted Turtles would take damage each turn, prevent that damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
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
impl Card for TuftedTurtles {
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
                    target: Some(CardQuery::from_id(self_id)),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, _card_id: &uuid::Uuid, effect: &Effect| {
                        Box::pin(async move {
                            let damage = match effect {
                                Effect::TakeDamage { damage, .. } if *damage > 0 => *damage,
                                _ => return Ok(vec![]),
                            };
                            let turtle = state.get_card(&self_id);
                            if turtle.has_ability(state, &Ability::DamageShieldUsed) {
                                return Ok(vec![]);
                            }
                            Ok(vec![
                                Effect::Heal {
                                    card_id: self_id,
                                    amount: damage,
                                },
                                Effect::AddAbilityCounter {
                                    card_id: self_id,
                                    counter: AbilityCounter {
                                        id: uuid::Uuid::new_v4(),
                                        ability: Ability::DamageShieldUsed,
                                        expires_on_effect: Some(EffectQuery::TurnEnd {
                                            player_id: None,
                                        }),
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TuftedTurtles::NAME, |owner_id: PlayerId| {
        Box::new(TuftedTurtles::new(owner_id))
    });
