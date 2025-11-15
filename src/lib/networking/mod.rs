pub mod client;

use crate::{card::CardType, game::State};
use serde::{Deserialize, Serialize};
use uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thresholds {
    pub fire: u8,
    pub water: u8,
    pub earth: u8,
    pub air: u8,
}

impl Thresholds {
    pub fn zero() -> Self {
        Self {
            fire: 0,
            water: 0,
            earth: 0,
            air: 0,
        }
    }
}

impl std::fmt::Display for Thresholds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fire: {}, Water: {}, Earth: {}, Air: {}",
            self.fire, self.water, self.earth, self.air
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Message {
    Connect,
    ConnectResponse {
        player_id: uuid::Uuid,
    },
    MatchCreated {
        player1: uuid::Uuid,
        player2: uuid::Uuid,
    },
    DrawCard {
        card_type: CardType,
        player_id: uuid::Uuid,
    },
    Sync {
        state: State,
    },
    CardPlayed {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        cell_id: u8,
    },
    Disconnect,
    EndTurn,
}
