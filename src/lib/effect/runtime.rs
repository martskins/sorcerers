use crate::{
    card::{HookId, HookSourceZones, HookTiming},
    effect::{Effect, EffectLogEmitter},
    game::{CardId, Game},
    state::State,
};

pub struct EffectEngine;

struct PendingHook {
    source_id: CardId,
    hook_id: HookId,
    source_zones: HookSourceZones,
}

impl EffectEngine {
    fn queues_resolved_hook_effects(effect: &Effect) -> bool {
        matches!(
            effect,
            Effect::TriggerGenesis { .. } | Effect::TriggerDeathrite { .. }
        )
    }

    async fn collect_hooks(
        state: &State,
        effect: &Effect,
        timing: HookTiming,
    ) -> anyhow::Result<Vec<PendingHook>> {
        let mut hooks = vec![];
        for card in state.cards.values() {
            if card.get_zone().is_in_play()
                && state.card_has_special_abilities_removed(card.get_id())
            {
                continue;
            }

            for hook in card.hooks(state)? {
                if !hook.source_zones.matches(card.get_zone()) {
                    continue;
                }

                if !hook.trigger.matches(effect, state).await? {
                    continue;
                }

                if hook.timing == timing {
                    hooks.push(PendingHook {
                        source_id: *card.get_id(),
                        hook_id: hook.id,
                        source_zones: hook.source_zones,
                    });
                }
            }
        }

        Ok(hooks)
    }

    async fn resolve_hooks(
        state: &mut State,
        effect: &Effect,
        hooks: &[PendingHook],
    ) -> anyhow::Result<()> {
        for hook in hooks {
            let Some(source) = state.cards.get(&hook.source_id) else {
                continue;
            };
            if !hook.source_zones.matches(source.get_zone()) {
                continue;
            }
            if source.get_zone().is_in_play()
                && state.card_has_special_abilities_removed(&hook.source_id)
            {
                continue;
            }

            let effects = source.resolve_hook(hook.hook_id, state, effect).await?;
            if Self::queues_resolved_hook_effects(effect) {
                state.queue(effects);
            } else {
                for effect in effects {
                    Box::pin(effect.apply(state)).await?;
                }
            }
        }

        Ok(())
    }

    async fn resolve_hook_replacements(
        state: &mut State,
        effect: &Effect,
        hooks: &[PendingHook],
    ) -> anyhow::Result<Vec<Effect>> {
        let mut replacements = vec![];
        for hook in hooks {
            let Some(source) = state.cards.get(&hook.source_id) else {
                continue;
            };
            if !hook.source_zones.matches(source.get_zone()) {
                continue;
            }
            if source.get_zone().is_in_play()
                && state.card_has_special_abilities_removed(&hook.source_id)
            {
                continue;
            }

            replacements.extend(source.resolve_hook(hook.hook_id, state, effect).await?);
        }

        Ok(replacements)
    }

    pub async fn drain_with_log(game: &mut Game) -> anyhow::Result<()> {
        while !game.state.effects.is_empty() {
            Self::step_with_log(game).await?;
        }

        Ok(())
    }

    pub async fn step_with_log(game: &mut Game) -> anyhow::Result<()> {
        if let Some(effect) = game.state.effects.pop_back() {
            let eliminated_before = game.state.eliminated_players.clone();
            let replace_hooks =
                Self::collect_hooks(&game.state, &effect, HookTiming::Replace).await?;
            let replacements =
                Self::resolve_hook_replacements(&mut game.state, &effect, &replace_hooks).await?;
            if !replacements.is_empty() {
                game.state.queue(replacements);
                return Ok(());
            }

            let before_hooks =
                Self::collect_hooks(&game.state, &effect, HookTiming::Before).await?;
            Self::resolve_hooks(&mut game.state, &effect, &before_hooks).await?;

            match effect.apply(&mut game.state).await {
                Ok(_) => {}
                Err(e) => {
                    println!("Error applying effect {:?}: {:?}", effect, e);
                }
            }

            EffectLogEmitter::emit(game, effect.clone()).await?;

            // Gather post hooks after effects are applied so that we get the latest state and
            // any state query done on the hooks function reflects the correct state of all
            // cards.
            let after_hooks = Self::collect_hooks(&game.state, &effect, HookTiming::After).await?;
            Self::resolve_hooks(&mut game.state, &effect, &after_hooks).await?;

            Game::dispell_auras(&mut game.state).await?;
            game.broadcast(&game.make_sync()?).await?;
            if game.state.eliminated_players != eliminated_before
                && let Some(messages) = game.game_over_messages()
            {
                for message in messages {
                    game.send_to_player(&message).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn drain_without_log(state: &mut State) -> anyhow::Result<()> {
        while !state.effects.is_empty() {
            Self::step_without_log(state).await?;
        }

        Ok(())
    }

    pub async fn step_without_log(state: &mut State) -> anyhow::Result<()> {
        if let Some(effect) = state.effects.pop_back() {
            let replace_hooks = Self::collect_hooks(state, &effect, HookTiming::Replace).await?;
            let replacements =
                Self::resolve_hook_replacements(state, &effect, &replace_hooks).await?;
            if !replacements.is_empty() {
                state.queue(replacements);
                return Ok(());
            }

            let before_hooks = Self::collect_hooks(state, &effect, HookTiming::Before).await?;
            Self::resolve_hooks(state, &effect, &before_hooks).await?;

            effect.apply(state).await?;

            let after_hooks = Self::collect_hooks(state, &effect, HookTiming::After).await?;
            Self::resolve_hooks(state, &effect, &after_hooks).await?;
        }

        Ok(())
    }
}
