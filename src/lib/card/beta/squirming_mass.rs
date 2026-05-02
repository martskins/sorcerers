use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::{Counter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct SquirmingMass {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SquirmingMass {
    pub const NAME: &'static str = "Squirming Mass";
    pub const DESCRIPTION: &'static str =
        "Whenever another nearby minion dies, Squirming Mass permanently gains its power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 3,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "EE"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SquirmingMass {
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
                trigger_on_effect: EffectQuery::BuryCard {
                    card: CardQuery::new().minions(),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, buried_id: &uuid::Uuid, _effect: &Effect| {
                        let buried_id = *buried_id;
                        Box::pin(async move {
                            let self_card = state.get_card(&self_id);
                            if !self_card.get_zone().is_in_play() {
                                return Ok(vec![]);
                            }
                            let buried_card = state.get_card(&buried_id);
                            let power = buried_card
                                .get_unit_base()
                                .map(|ub| ub.power as i16)
                                .unwrap_or(0);
                            if power <= 0 {
                                return Ok(vec![]);
                            }
                            Ok(vec![Effect::AddCounter {
                                card_id: self_id,
                                counter: Counter::new(power, 0, None),
                            }])
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
    (SquirmingMass::NAME, |owner_id: PlayerId| {
        Box::new(SquirmingMass::new(owner_id))
    });
