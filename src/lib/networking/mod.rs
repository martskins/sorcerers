pub mod client;

use crate::card::{Card, CardType};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Connect,
    ConnectResponse {
        player_id: uuid::Uuid,
    },
    MatchCreated {
        player1: uuid::Uuid,
        player2: uuid::Uuid,
    },
    Sync {
        cards: Vec<Card>,
    },
    Disconnect,
    PlayCard {
        card_id: u32,
    },
    EndTurn,
}
