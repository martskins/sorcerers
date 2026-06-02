use crate::{
    card::{Ability, UnitBase},
    effect::Effect,
    game::CardId,
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

pub type EffectCallback = Arc<
    dyn Sync
        + Send
        + for<'a> Fn(
            &'a State,
            &'a CardId,
            &'a Effect,
        )
            -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + 'a>>,
>;

#[derive(Clone)]
pub struct DeferredEffect {
    pub trigger_on_effect: EffectQuery,
    pub expires_on_effect: Option<EffectQuery>,
    pub on_effect: EffectCallback,
    pub multitrigger: bool,
}

impl std::fmt::Debug for DeferredEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredEffect")
            .field("trigger_on_effect", &self.trigger_on_effect)
            .field("expires_on_effect", &self.expires_on_effect)
            .finish()
    }
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
        let deferred_effects = state.deferred_effects().to_vec();
        for (idx, de) in deferred_effects.iter().enumerate() {
            let source_ids = de.trigger_on_effect.source_ids(effect, state).await?;
            if !source_ids.is_empty() {
                let source_ids = if de.multitrigger {
                    source_ids
                } else {
                    source_ids.into_iter().take(1).collect()
                };
                for source_id in source_ids {
                    let effects = (de.on_effect)(state, &source_id, effect).await?;
                    state.queue(effects);
                }

                if !de.multitrigger {
                    effects_to_remove.push(idx);
                }
            } else if let Some(expires_on_effect) = &de.expires_on_effect
                && expires_on_effect.matches(effect, state).await?
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
