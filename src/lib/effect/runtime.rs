use crate::{
    card::{HookId, HookSourceZones, HookTiming},
    effect::{Effect, EffectLogEmitter},
    game::{CardId, Game},
    state::{OngoingEffect, State},
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
        for card in state.all_cards() {
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

            for ongoing_effect in &state.ongoing_effects {
                if let OngoingEffect::GrantHook {
                    affected_cards: cards,
                    hook,
                    hook_resolver,
                } = &ongoing_effect.effect
                    && cards.matches(card.get_id(), state)
                    && hook.trigger.matches(effect, state).await?
                    && hook.source_zones.matches(card.get_zone())
                    && hook.timing == timing
                {
                    hooks.push(PendingHook {
                        source_id: *hook_resolver,
                        hook_id: hook.id,
                        source_zones: hook.source_zones.clone(),
                    });
                }
            }
        }

        Ok(hooks)
    }

    // TODO: Find a way to get rid of this duplication.
    async fn resolve_hooks_without_log(
        state: &mut State,
        effect: &Effect,
        hooks: &[PendingHook],
    ) -> anyhow::Result<()> {
        for hook in hooks {
            let Some(source) = state.try_get_card(&hook.source_id) else {
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

    async fn resolve_hooks(
        game: &mut Game,
        effect: &Effect,
        hooks: &[PendingHook],
    ) -> anyhow::Result<()> {
        for hook in hooks {
            let Some(source) = game.state.try_get_card(&hook.source_id) else {
                continue;
            };
            if !hook.source_zones.matches(source.get_zone()) {
                continue;
            }
            if source.get_zone().is_in_play()
                && game
                    .state
                    .card_has_special_abilities_removed(&hook.source_id)
            {
                continue;
            }

            let effects = source
                .resolve_hook(hook.hook_id, &game.state, effect)
                .await?;
            if Self::queues_resolved_hook_effects(effect) {
                game.state.queue(effects);
            } else {
                for effect in effects {
                    Box::pin(effect.apply(&mut game.state)).await?;
                    EffectLogEmitter::emit(game, effect.clone()).await?;
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
            let Some(source) = state.try_get_card(&hook.source_id) else {
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
            let mut replacements =
                Self::resolve_hook_replacements(&mut game.state, &effect, &replace_hooks).await?;
            if !replacements.is_empty() {
                if let Some(first_effect) = replacements.pop() {
                    // Process first effect in the replacements vec, assuming that if an effect is
                    // replaced with the same effect with different parameters, it will be on the
                    // first position.
                    //
                    // TODO: Probably a better way to do this is to skip this specific effect in the
                    // card that triggered the replacement. We may need to include an id in effects.
                    // Some cards that may benefit from this are Doomsday Prophet, Critical Strike,
                    // Drums of Doom.
                    first_effect.apply(&mut game.state).await?;

                    EffectLogEmitter::emit(game, first_effect.clone()).await?;

                    let after_hooks =
                        Self::collect_hooks(&game.state, &first_effect, HookTiming::After).await?;
                    Self::resolve_hooks(game, &first_effect, &after_hooks).await?;

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

                game.state.queue(replacements);
                return Ok(());
            }

            let before_hooks =
                Self::collect_hooks(&game.state, &effect, HookTiming::Before).await?;
            Self::resolve_hooks(game, &effect, &before_hooks).await?;

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
            Self::resolve_hooks(game, &effect, &after_hooks).await?;

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
            Self::resolve_hooks_without_log(state, &effect, &before_hooks).await?;

            effect.apply(state).await?;

            let after_hooks = Self::collect_hooks(state, &effect, HookTiming::After).await?;
            Self::resolve_hooks_without_log(state, &effect, &after_hooks).await?;
        }

        Ok(())
    }
}
