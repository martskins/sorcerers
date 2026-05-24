use std::{future::Future, pin::Pin, sync::Arc};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Battlemage {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
    trigger_registered: bool,
}

impl Battlemage {
    pub const NAME: &'static str = "Battlemage";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r Whenever Battlemage attacks and kills an enemy, you may draw a spell.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
            trigger_registered: false,
        }
    }

    fn register_trigger(&self) -> Vec<Effect> {
        let battlemage_id = *self.get_id();
        vec![
            Effect::SetCardData {
                card_id: battlemage_id,
                data: std::sync::Arc::new(true),
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::DamageDealt {
                        source: Some(CardQuery::from_id(battlemage_id)),
                        target: None,
                    },
                    expires_on_effect: None,
                    on_effect: Arc::new(
                        move |state: &State, damaged_id: &uuid::Uuid, effect: &Effect| {
                            let damaged_id = *damaged_id;
                            Box::pin(async move {
                                let battlemage = state.get_card(&battlemage_id);
                                if !battlemage.get_zone().is_in_play() {
                                    return Ok(vec![]);
                                }

                                let Effect::TakeDamage { from, .. } = effect else {
                                    return Ok(vec![]);
                                };
                                if from != &battlemage_id {
                                    return Ok(vec![]);
                                }

                                let killed_enemy = state.effects.iter().any(|queued| {
                                    matches!(queued, Effect::KillMinion { card_id, killer_id, from_attack: true }
                                        if *card_id == damaged_id && *killer_id == battlemage_id)
                                });
                                if !killed_enemy {
                                    return Ok(vec![]);
                                }

                                let controller = battlemage.get_controller_id(state);
                                if state.get_card(&damaged_id).get_controller_id(state)
                                    == controller
                                {
                                    return Ok(vec![]);
                                }

                                let draw = yes_or_no_source(
                                    &controller,
                                    state,
                                    "Draw a spell?",
                                    Some(battlemage_id),
                                )
                                .await?;
                                if draw {
                                    Ok(vec![Effect::DrawCard {
                                        player_id: controller,
                                        count: 1,
                                        kind: DrawKind::Spell,
                                    }])
                                } else {
                                    Ok(vec![])
                                }
                            })
                                as Pin<
                                    Box<
                                        dyn Future<Output = anyhow::Result<Vec<Effect>>>
                                            + Send
                                            + '_,
                                    >,
                                >
                        },
                    ),
                    multitrigger: true,
                },
            },
        ]
    }
}

#[async_trait::async_trait]
impl Card for Battlemage {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(trigger_registered) = data.downcast_ref::<bool>() {
            self.trigger_registered = *trigger_registered;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Battlemage"))
        }
    }

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(self.register_trigger())
    }

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        if self.trigger_registered {
            return Ok(vec![]);
        }

        Ok(self.register_trigger())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Battlemage::NAME, |owner_id: PlayerId| {
    Box::new(Battlemage::new(owner_id))
});
