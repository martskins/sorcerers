use std::sync::Arc;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WarpSpasm {
    card_base: CardBase,
}

impl WarpSpasm {
    pub const NAME: &'static str = "Warp Spasm";
    pub const DESCRIPTION: &'static str = "This turn, double an allied minion's power, and whenever it attacks and kills a unit, it untaps. At the end of the turn, it dies.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FFF"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WarpSpasm {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for WarpSpasm {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(target_id) = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_play()
            .with_prompt("Pick an allied minion")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let power = state
            .get_card(&target_id)
            .get_power(state)?
            .unwrap_or_default() as i16;

        Ok(vec![
            Effect::AddCounter {
                card_id: target_id,
                counter: Counter {
                    id: uuid::Uuid::new_v4(),
                    power,
                    toughness: 0,
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                },
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::UnitKilled {
                        unit: CardQuery::new().minions(),
                        killer: Some(self.get_id().into()),
                        from_attack: Some(true),
                    },
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                    on_effect: Arc::new(move |_state: &State, _source, _effect: &Effect| {
                        Box::pin(async move {
                            Ok(vec![Effect::SetTapped {
                                card_id: target_id,
                                tapped: false,
                            }])
                        })
                    }),
                    multitrigger: true,
                },
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::TurnEnd { player_id: None },
                    expires_on_effect: None,
                    on_effect: Arc::new(move |_state: &State, _source, _effect: &Effect| {
                        Box::pin(async move {
                            Ok(vec![Effect::KillMinion {
                                card_id: target_id,
                                killer_id: target_id,
                                from_attack: false,
                            }])
                        })
                    }),
                    multitrigger: false,
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WarpSpasm::NAME, |owner_id: PlayerId| {
    Box::new(WarpSpasm::new(owner_id))
});
