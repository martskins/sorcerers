use crate::{game::Game, networking::message::ServerMessage};
use chrono::Utc;

use super::Effect;

#[derive(Debug, Clone)]
pub struct LoggedEffect {
    pub effect: Effect,
    pub turn: usize,
}

impl LoggedEffect {
    pub fn new(effect: Effect, turn: usize) -> Self {
        Self { effect, turn }
    }
}

pub struct EffectLogEmitter;

impl EffectLogEmitter {
    pub async fn emit(game: &mut Game, effect: Effect) -> anyhow::Result<()> {
        let description = effect.description(&game.state).await.ok().flatten();

        // Show the card face to all players when a card is played from hand.
        // When CardPlayed is sent, skip the LogEvent to avoid showing the
        // same description twice on the client.
        let is_card_played = effect.played_card_id().is_some();
        if let Some(card_id) = effect.played_card_id() {
            game.broadcast(&ServerMessage::CardPlayed {
                card_id,
                description: description.clone().unwrap_or_default(),
            })
            .await?;
        }

        if !is_card_played && let Some(desc) = description {
            game.broadcast(&ServerMessage::LogEvent {
                id: uuid::Uuid::new_v4(),
                description: desc,
                datetime: Utc::now(),
            })
            .await?;
        }

        if let Ok(Some(sound_effect)) = effect.sound_effect().await {
            game.broadcast(&ServerMessage::PlaySoundEffect {
                player_id: None,
                sound_effect,
            })
            .await?;
        }

        let turn = game.state.turns;
        game.state
            .effect_log_mut()
            .push(LoggedEffect::new(effect, turn));

        Ok(())
    }
}
