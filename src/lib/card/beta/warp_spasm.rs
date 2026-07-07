use crate::prelude::*;

const ATTACK_AND_KILL_HOOK: HookId = 1;
const END_OF_TURN_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct WarpSpasm {
    card_base: CardBase,
    target_id: Option<CardId>,
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
            target_id: None,
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(data) = data.downcast_ref::<CardId>() {
            self.target_id = Some(*data);
        }

        Ok(())
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        let Some(target_id) = self.target_id else {
            return Ok(vec![]);
        };

        Ok(vec![
            Hook {
                id: ATTACK_AND_KILL_HOOK,
                trigger: EffectQuery::UnitKilled {
                    unit: Box::new(CardQuery::new().units()),
                    killer: Some(target_id.into()),
                    from_attack: Some(true),
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::Any,
            },
            Hook {
                id: END_OF_TURN_HOOK,
                trigger: EffectQuery::TurnEnd { player_id: None },
                timing: HookTiming::After,
                source_zones: HookSourceZones::Any,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ATTACK_AND_KILL_HOOK => {
                let Some(target_id) = self.target_id else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::SetTapped {
                    card_id: target_id,
                    tapped: false,
                }])
            }
            END_OF_TURN_HOOK => {
                let Some(target_id) = self.target_id else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::KillMinion {
                    card_id: target_id,
                    killer_id: *self.get_id(),
                    from_attack: false,
                }])
            }
            _ => Ok(vec![]),
        }
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
            .pick(&controller_id, state)
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
                    hook_id: ATTACK_AND_KILL_HOOK,
                    card_id: *self.get_id(),
                    timing: HookTiming::After,
                    trigger_on_effect: EffectQuery::UnitKilled {
                        unit: Box::new(CardQuery::new().minions()),
                        killer: Some(self.get_id().into()),
                        from_attack: Some(true),
                    },
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                    trigger_times: None,
                },
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    hook_id: END_OF_TURN_HOOK,
                    card_id: *self.get_id(),
                    timing: HookTiming::After,
                    trigger_on_effect: EffectQuery::TurnEnd { player_id: None },
                    expires_on_effect: None,
                    trigger_times: Some(1),
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WarpSpasm::NAME, |owner_id: PlayerId| {
    Box::new(WarpSpasm::new(owner_id))
});
