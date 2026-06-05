use std::{future::Future, pin::Pin, sync::Arc};

use crate::prelude::*;

const ON_SUMMON_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct ScreamingSkull {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl ScreamingSkull {
    pub const NAME: &'static str = "Screaming Skull";
    pub const DESCRIPTION: &'static str = "Whenever bearer attacks and kills an enemy, it untaps.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for ScreamingSkull {}

#[async_trait::async_trait]
impl Card for ScreamingSkull {
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

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: ON_SUMMON_HOOK,
            trigger: EffectQuery::OneOf(vec![
                EffectQuery::PlayCard {
                    card: self.get_id().into(),
                    spellcaster: None,
                },
                EffectQuery::SummonCard {
                    card: self.get_id().into(),
                },
            ]),
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ON_SUMMON_HOOK => {
                let skull_id = *self.get_id();
                Ok(vec![Effect::AddDeferredEffect {
                    effect: DeferredEffect {
                        trigger_on_effect: EffectQuery::DamageDealt {
                            source: None,
                            target: None,
                        },
                        expires_on_effect: Some(EffectQuery::BuryCard {
                            card: CardQuery::from_id(skull_id),
                        }),
                        on_effect: Arc::new(
                            move |state: &State, _: &uuid::Uuid, effect: &Effect| {
                                Box::pin(async move {
                                    let Effect::TakeDamage {
                                        card_id: damaged_id,
                                        from,
                                        ..
                                    } = effect
                                    else {
                                        return Ok(vec![]);
                                    };

                                    let skull = state.get_card(&skull_id);
                                    if !skull.get_zone().is_in_play() {
                                        return Ok(vec![]);
                                    }

                                    let Some(bearer_id) = skull.get_bearer_id()? else {
                                        return Ok(vec![]);
                                    };
                                    if from != &bearer_id {
                                        return Ok(vec![]);
                                    }

                                    let killed_enemy = state.effects.iter().any(|queued| {
                                        matches!(queued, Effect::KillMinion { card_id, killer_id, from_attack: true }
                                if card_id == damaged_id && *killer_id == bearer_id)
                                    });
                                    if !killed_enemy {
                                        return Ok(vec![]);
                                    }

                                    let bearer = state.get_card(&bearer_id);
                                    let bearer_controller = bearer.get_controller_id(state);
                                    let damaged = state.get_card(damaged_id);
                                    if damaged.get_controller_id(state) == bearer_controller {
                                        return Ok(vec![]);
                                    }

                                    Ok(vec![Effect::SetTapped {
                                        card_id: bearer_id,
                                        tapped: false,
                                    }])
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
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScreamingSkull::NAME, |owner_id: PlayerId| {
        Box::new(ScreamingSkull::new(owner_id))
    });
