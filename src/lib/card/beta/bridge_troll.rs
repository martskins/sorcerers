use crate::prelude::*;

const DRAIN_MANA_HOOK: HookId = 1;
const GIVE_MANA_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct BridgeTroll {
    unit_base: UnitBase,
    card_base: CardBase,
    opponent_mana: Option<u8>,
}

impl BridgeTroll {
    pub const NAME: &'static str = "Bridge Troll";
    pub const DESCRIPTION: &'static str = "Whenever an enemy attacks Bridge Troll, they must spend all of their remaining mana to give to you on your next turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "W"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            opponent_mana: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for BridgeTroll {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(data) = data.downcast_ref::<u8>() {
            self.opponent_mana = Some(*data);
        }

        Ok(())
    }

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![
            Hook {
                id: DRAIN_MANA_HOOK,
                trigger: EffectQuery::Attack {
                    attacker: CardQuery::new().not_controlled_by(&self.get_controller_id(state)),
                    defender: Some(self.get_id().into()),
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::Any,
            },
            Hook {
                id: GIVE_MANA_HOOK,
                trigger: EffectQuery::TurnStart {
                    player_id: Some(self.get_controller_id(state)),
                },
                timing: HookTiming::After,
                source_zones: HookSourceZones::Any,
            },
        ])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GIVE_MANA_HOOK => {
                let Some(amount) = self.opponent_mana else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::AdjustMana {
                    player_id: self.get_controller_id(state),
                    amount: amount as i8,
                }])
            }
            DRAIN_MANA_HOOK => {
                let Effect::DeclareAttack { attacker_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let attacker = state.get_card(attacker_id);
                let attacker_controller = attacker.get_controller_id(state);
                let my_controller = self.get_controller_id(state);
                let opponent_mana =
                    *state.player_mana.get(&attacker_controller).unwrap_or(&0) as i8;

                Ok(vec![
                    Effect::AdjustMana {
                        player_id: attacker_controller,
                        amount: -opponent_mana,
                    },
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: Arc::new(opponent_mana),
                    },
                    // TODO: This doesn't allow for multiple pending mana transfers to be queued. If two
                    // attacks to bridge troll happen on the same turn, the second one will override
                    // the first one.
                    Effect::AddDeferredEffect {
                        effect: DeferredEffect {
                            hook_id: GIVE_MANA_HOOK,
                            card_id: *self.get_id(),
                            trigger_on_effect: EffectQuery::TurnStart {
                                player_id: Some(my_controller),
                            },
                            expires_on_effect: None,
                            trigger_times: Some(1),
                        },
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BridgeTroll::NAME, |owner_id: PlayerId| {
    Box::new(BridgeTroll::new(owner_id))
});
