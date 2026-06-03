use crate::{
    card::{HookAction, HookTiming},
    effect::EffectLogEmitter,
    game::Game,
    state::State,
};

pub struct EffectEngine;

impl EffectEngine {
    pub async fn drain_with_log(game: &mut Game) -> anyhow::Result<()> {
        while !game.state.effects.is_empty() {
            if let Some(effect) = game.state.effects.pop_back() {
                // Gather hooks into a before and after vec.
                let mut before_hooks = vec![];
                let mut after_hooks = vec![];
                for card in game.state.cards.values() {
                    for hook in card.hooks(&game.state).await? {
                        if !hook.trigger.matches(&effect, &game.state).await? {
                            continue;
                        }

                        match hook.timing {
                            HookTiming::Before => before_hooks.push(hook.action),
                            HookTiming::After => after_hooks.push(hook.action),
                        }
                    }
                }

                let eliminated_before = game.state.eliminated_players.clone();
                for hook_action in &before_hooks {
                    match hook_action {
                        HookAction::Effects(effects) => {
                            for effect in effects {
                                Box::pin(effect.apply(&mut game.state)).await?;
                            }
                        }
                        HookAction::Replace(_effects) => todo!(),
                    }
                }

                match effect.apply(&mut game.state).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error applying effect {:?}: {:?}", effect, e);
                    }
                }

                EffectLogEmitter::emit(game, effect.clone()).await?;

                for hook_action in &after_hooks {
                    match hook_action {
                        HookAction::Effects(effects) => {
                            for effect in effects {
                                Box::pin(effect.apply(&mut game.state)).await?;
                            }
                        }
                        HookAction::Replace(_effects) => todo!(),
                    }
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
                effect.apply(state).await?;
            }
        }

        Ok(())
    }
}
