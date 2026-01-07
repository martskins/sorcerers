use crate::game::PlayerId;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GameError {
    PlayerDisconnected(PlayerId),
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::PlayerDisconnected(player_id) => {
                write!(f, "Player {:?} disconnected", player_id)
            }
        }
    }
}
