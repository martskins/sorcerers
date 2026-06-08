use crate::{
    card::{Ability, HookId, UnitBase},
    effect::Effect,
    game::{CardId, PlayerId},
    query::{CardQuery, EffectQuery},
    state::State,
    zone::Zone,
};
use std::{future::Future, pin::Pin, sync::Arc};

pub type EffectReplacementCallback = Arc<
    dyn Sync
        + Send
        + for<'a> Fn(
            &'a State,
            &'a mut Effect,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>,
>;

#[derive(Debug, Clone)]
pub struct DeferredEffect {
    pub hook_id: HookId,
    pub card_id: CardId,
    pub trigger_on_effect: EffectQuery,
    pub expires_on_effect: Option<EffectQuery>,
    pub trigger_times: Option<u8>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum TemporaryEffect {
    Animate {
        card_id: CardId,
        unit_base: UnitBase,
        expires_on_effect: EffectQuery,
    },
    GrantAbility {
        ability: Ability,
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
    },
    MakePlayable {
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
        by_player: crate::game::PlayerId,
    },
    IgnoreCostThresholds {
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
        for_player: crate::game::PlayerId,
    },
    ModifyEffect {
        trigger_on_effect: EffectQuery,
        expires_on_effect: EffectQuery,
        on_effect: EffectReplacementCallback,
    },
    ConnectSites {
        sites: Vec<Zone>,
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
    },
    ControllerOverride {
        controller_id: PlayerId,
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
    },
}

impl std::fmt::Debug for TemporaryEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Animate {
                card_id,
                unit_base,
                expires_on_effect,
            } => f
                .debug_struct("Animate")
                .field("card_id", card_id)
                .field("unit_base", unit_base)
                .field("expires_on_effect", expires_on_effect)
                .finish(),
            Self::GrantAbility {
                ability,
                affected_cards,
                expires_on_effect,
            } => f
                .debug_struct("GrantAbility")
                .field("ability", ability)
                .field("affected_cards", affected_cards)
                .field("expires_on_effect", expires_on_effect)
                .finish(),
            Self::MakePlayable {
                affected_cards,
                expires_on_effect,
                by_player,
            } => f
                .debug_struct("MakePlayable")
                .field("affected_cards", affected_cards)
                .field("expires_on_effect", expires_on_effect)
                .field("by_player", by_player)
                .finish(),
            Self::IgnoreCostThresholds {
                affected_cards,
                expires_on_effect,
                for_player,
            } => f
                .debug_struct("IgnoreCostThresholds")
                .field("affected_cards", affected_cards)
                .field("expires_on_effect", expires_on_effect)
                .field("for_player", for_player)
                .finish(),
            Self::ModifyEffect {
                trigger_on_effect, ..
            } => f
                .debug_struct("ModifyEffect")
                .field("trigger_on_effect", trigger_on_effect)
                .finish(),
            Self::ConnectSites {
                sites,
                affected_cards,
                expires_on_effect,
            } => f
                .debug_struct("ConnectSites")
                .field("sites", sites)
                .field("affected_cards", affected_cards)
                .field("expires_on_effect", expires_on_effect)
                .finish(),
            Self::ControllerOverride {
                ..
                // TODO: Finish Debug impl
            } => f.debug_struct("ControllerOverride").finish(),
        }
    }
}

impl TemporaryEffect {
    pub fn affected_cards(&self, state: &State) -> Vec<CardId> {
        match self {
            TemporaryEffect::Animate { card_id, .. } => vec![*card_id],
            TemporaryEffect::GrantAbility { affected_cards, .. } => affected_cards.all(state),
            TemporaryEffect::MakePlayable { affected_cards, .. } => {
                affected_cards.clone().including_not_in_play().all(state)
            }
            TemporaryEffect::IgnoreCostThresholds { affected_cards, .. } => {
                affected_cards.all(state)
            }
            TemporaryEffect::ModifyEffect { .. } => vec![],
            TemporaryEffect::ConnectSites { .. } => vec![],
            TemporaryEffect::ControllerOverride { affected_cards, .. } => affected_cards.all(state),
        }
    }

    pub fn expires_on_effect(&self) -> Option<&EffectQuery> {
        match self {
            TemporaryEffect::Animate {
                expires_on_effect, ..
            }
            | TemporaryEffect::GrantAbility {
                expires_on_effect, ..
            }
            | TemporaryEffect::MakePlayable {
                expires_on_effect, ..
            }
            | TemporaryEffect::IgnoreCostThresholds {
                expires_on_effect, ..
            }
            | TemporaryEffect::ModifyEffect {
                expires_on_effect, ..
            }
            | TemporaryEffect::ConnectSites {
                expires_on_effect, ..
            }
            | TemporaryEffect::ControllerOverride {
                expires_on_effect, ..
            } => Some(expires_on_effect),
        }
    }
}

pub struct EffectLifecycle;

impl EffectLifecycle {
    pub async fn modify_effect(state: &mut State, effect: &mut Effect) -> anyhow::Result<()> {
        let temporary_effects = state.temporary_effects().to_vec();
        for te in temporary_effects {
            match te {
                TemporaryEffect::ModifyEffect {
                    trigger_on_effect,
                    on_effect,
                    ..
                } if trigger_on_effect.matches(effect, state).await? => {
                    on_effect(state, effect).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn after_resolved_effect(state: &mut State, effect: &Effect) -> anyhow::Result<()> {
        Self::process_deferred_effects(state, effect).await?;
        Self::expire_temporary_effects(state, effect).await?;
        Ok(())
    }

    async fn process_deferred_effects(state: &mut State, effect: &Effect) -> anyhow::Result<()> {
        let mut effects_to_remove = vec![];
        let mut deferred_effects = state.deferred_effects().to_vec();
        for (idx, de) in deferred_effects.iter_mut().enumerate() {
            if de.trigger_on_effect.matches(effect, state).await? {
                if let Some(v) = de.trigger_times.as_mut() {
                    *v -= 1
                }

                let Some(card) = state.cards.get(&de.card_id) else {
                    return Err(anyhow::anyhow!("failed to get card by id"));
                };

                let effects = card.resolve_hook(de.hook_id, state, effect).await?;
                state.queue(effects);

                if de.trigger_times.is_some_and(|tt| tt == 0) {
                    effects_to_remove.push(idx);
                }
            }

            if let Some(ee) = &de.expires_on_effect
                && ee.matches(effect, state).await?
            {
                effects_to_remove.push(idx);
            }
        }

        effects_to_remove.reverse();
        for idx in effects_to_remove {
            state.deferred_effects_mut().remove(idx);
        }

        Ok(())
    }

    async fn expire_temporary_effects(state: &mut State, effect: &Effect) -> anyhow::Result<()> {
        let snapshot = state.clone();
        let mut retained_effects = vec![];
        for te in state.temporary_effects() {
            let should_retain = match te.expires_on_effect() {
                Some(expiry_effect) => !expiry_effect.matches(effect, &snapshot).await?,
                None => true,
            };

            if should_retain {
                retained_effects.push(te.clone());
            }
        }

        *state.temporary_effects_mut() = retained_effects;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct EffectState {
    queue: std::collections::VecDeque<Effect>,
    log: Vec<crate::effect::LoggedEffect>,
    temporary: Vec<TemporaryEffect>,
    deferred: Vec<DeferredEffect>,
}

impl EffectState {
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn pop_back(&mut self) -> Option<Effect> {
        self.queue.pop_back()
    }

    pub fn get_queue_debug_data(&self) -> Vec<(String, String)> {
        self.queue
            .iter()
            .map(|e| {
                let full = format!("{:?}", e);
                let name = full
                    .split([' ', '{', '('])
                    .next()
                    .unwrap_or(&full)
                    .to_string();
                (name, full)
            })
            .collect()
    }

    pub fn push_back(&mut self, effect: Effect) {
        self.queue.push_back(effect);
    }

    pub fn push_front(&mut self, effect: Effect) {
        self.queue.push_front(effect);
    }

    pub fn extend(&mut self, effects: impl IntoIterator<Item = Effect>) {
        self.queue.extend(effects);
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, Effect> {
        self.queue.iter()
    }

    pub fn log(&self) -> &[crate::effect::LoggedEffect] {
        &self.log
    }

    pub fn log_mut(&mut self) -> &mut Vec<crate::effect::LoggedEffect> {
        &mut self.log
    }

    pub fn temporary(&self) -> &[TemporaryEffect] {
        &self.temporary
    }

    pub fn temporary_mut(&mut self) -> &mut Vec<TemporaryEffect> {
        &mut self.temporary
    }

    pub fn deferred(&self) -> &[DeferredEffect] {
        &self.deferred
    }

    pub fn deferred_mut(&mut self) -> &mut Vec<DeferredEffect> {
        &mut self.deferred
    }
}

impl IntoIterator for EffectState {
    type Item = Effect;
    type IntoIter = std::collections::vec_deque::IntoIter<Effect>;

    fn into_iter(self) -> Self::IntoIter {
        self.queue.into_iter()
    }
}
