use crate::{
    card::{HookAction, HookTiming},
    effect::EffectLogEmitter,
    game::Game,
    state::State,
};

pub struct EffectEngine;

impl EffectEngine {
    async fn matching_hooks(
        state: &State,
        effect: &crate::effect::Effect,
    ) -> anyhow::Result<(Vec<HookAction>, Vec<HookAction>)> {
        let mut before_hooks = vec![];
        let mut after_hooks = vec![];
        for card in state.cards.values() {
            for hook in card.hooks(state).await? {
                if !hook.trigger.matches(effect, state).await? {
                    continue;
                }

                match hook.timing {
                    HookTiming::Before => before_hooks.push(hook.action),
                    HookTiming::After => after_hooks.push(hook.action),
                }
            }
        }

        Ok((before_hooks, after_hooks))
    }

    async fn apply_hook_action(
        state: &mut State,
        effect: &crate::effect::Effect,
        hook_action: &HookAction,
    ) -> anyhow::Result<()> {
        match hook_action {
            HookAction::Effects(effects) => {
                for effect in effects {
                    Box::pin(effect.apply(state)).await?;
                }
            }
            HookAction::Callback(callback) => {
                let effects = callback(state, effect).await?;
                for effect in effects {
                    Box::pin(effect.apply(state)).await?;
                }
            }
            HookAction::Replace(_effects) => todo!(),
        }

        Ok(())
    }

    pub async fn drain_with_log(game: &mut Game) -> anyhow::Result<()> {
        while !game.state.effects.is_empty() {
            if let Some(effect) = game.state.effects.pop_back() {
                let (before_hooks, after_hooks) =
                    Self::matching_hooks(&game.state, &effect).await?;

                let eliminated_before = game.state.eliminated_players.clone();
                for hook_action in &before_hooks {
                    Self::apply_hook_action(&mut game.state, &effect, hook_action).await?;
                }

                match effect.apply(&mut game.state).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error applying effect {:?}: {:?}", effect, e);
                    }
                }

                EffectLogEmitter::emit(game, effect.clone()).await?;

                for hook_action in &after_hooks {
                    Self::apply_hook_action(&mut game.state, &effect, hook_action).await?;
                }

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
        }

        Ok(())
    }

    pub async fn drain_without_log(state: &mut State) -> anyhow::Result<()> {
        while !state.effects.is_empty() {
            if let Some(effect) = state.effects.pop_back() {
                let (before_hooks, after_hooks) = Self::matching_hooks(state, &effect).await?;
                for hook_action in &before_hooks {
                    Self::apply_hook_action(state, &effect, hook_action).await?;
                }

                effect.apply(state).await?;

                for hook_action in &after_hooks {
                    Self::apply_hook_action(state, &effect, hook_action).await?;
                }
            }
        }

        Ok(())
    }
}
