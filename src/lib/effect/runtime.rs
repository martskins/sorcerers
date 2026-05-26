use crate::{effect::EffectLogEmitter, game::Game, state::State};

pub struct EffectEngine;

impl EffectEngine {
    pub async fn drain_with_log(game: &mut Game) -> anyhow::Result<()> {
        while !game.state.effects.is_empty() {
            let effect = game.state.effects.pop_back();
            if let Some(effect) = effect {
                let eliminated_before = game.state.eliminated_players.clone();
                match effect.apply(&mut game.state).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error applying effect {:?}: {:?}", effect, e);
                    }
                }

                EffectLogEmitter::emit(game, effect).await?;

                Game::dispell_auras(&mut game.state).await?;
                game.broadcast(&game.make_sync()?).await?;
                if game.state.eliminated_players != eliminated_before
                    && let Some(message) = game.game_over_message()
                {
                    game.broadcast(&message).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn drain_without_log(state: &mut State) -> anyhow::Result<()> {
        while !state.effects.is_empty() {
            if state.waiting_for_input {
                return Ok(());
            }

            if let Some(effect) = state.effects.pop_back() {
                effect.apply(state).await?;
            }
        }

        Ok(())
    }
}
