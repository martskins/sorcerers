pub mod client;

use crate::card::{Card, CardType};
use serde::{Deserialize, Serialize};
use uuid;

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
        cards: Vec<Card>,
    },
    CardPlayed {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        cell_id: u8,
    },
    Disconnect,
    EndTurn,
}
