use std::{future::Future, pin::Pin, sync::Arc};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BridgeTroll {
    unit_base: UnitBase,
    card_base: CardBase,
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        let self_id = *self.get_id();
        Ok(vec![Hook {
            trigger: EffectQuery::Attack {
                attacker: CardQuery::new(),
                defender: Some(self.get_id().into()),
            },
            timing: HookTiming::Before,
            action: HookAction::Callback(Arc::new(move |state: &State, effect: &Effect| {
                Box::pin(async move {
                    let Effect::Attack { attacker_id, .. } = effect else {
                        return Ok(vec![]);
                    };

                    if state.card_has_special_abilities_removed(&self_id) {
                        return Ok(vec![]);
                    }

                    let attacker = state.get_card(attacker_id);
                    let attacker_controller = attacker.get_controller_id(state);
                    let bridge_troll = state.get_card(&self_id);
                    let my_controller = bridge_troll.get_controller_id(state);

                    let opponent_mana =
                        *state.player_mana.get(&attacker_controller).unwrap_or(&0) as i8;

                    Ok(vec![
                        Effect::AdjustMana {
                            player_id: attacker_controller,
                            mana: -opponent_mana,
                        },
                        Effect::AddDeferredEffect {
                            effect: DeferredEffect {
                                trigger_on_effect: EffectQuery::TurnStart {
                                    player_id: Some(my_controller),
                                },
                                expires_on_effect: None,
                                on_effect: Arc::new(
                                    move |_: &State, _: &uuid::Uuid, _: &Effect| {
                                        Box::pin(async move {
                                            Ok(vec![Effect::AdjustMana {
                                                player_id: my_controller,
                                                mana: opponent_mana,
                                            }])
                                        })
                                    },
                                ),
                                multitrigger: false,
                            },
                        },
                    ])
                })
                    as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
            })),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BridgeTroll::NAME, |owner_id: PlayerId| {
    Box::new(BridgeTroll::new(owner_id))
});
