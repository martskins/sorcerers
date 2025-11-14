pub mod client;

use std::collections::HashMap;

use crate::card::{Card, CardType};
use serde::{Deserialize, Serialize};
use uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Thresholds {
    pub fire: u8,
    pub water: u8,
    pub earth: u8,
    pub air: u8,
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
        cards: Vec<Card>,
        mana: HashMap<uuid::Uuid, u8>,
        thresholds: HashMap<uuid::Uuid, Thresholds>,
    },
    CardPlayed {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        cell_id: u8,
    },
    Disconnect,
    EndTurn,
}
