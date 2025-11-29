pub mod client;

use std::net::SocketAddr;

use crate::{
    card::{CardType, Target},
    game::State,
};
use serde::{Deserialize, Serialize};
use uuid;

#[derive(Debug, Clone)]
pub enum Socket {
    SocketAddr(SocketAddr),
    Noop,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thresholds {
    pub fire: u8,
    pub water: u8,
    pub earth: u8,
    pub air: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    Fire,
    Water,
    Earth,
    Air,
}

impl Thresholds {
    pub fn parse(input: &str) -> Self {
        let mut threshold = Self::zero();
        for c in input.to_uppercase().chars() {
            match c {
                'F' => {
                    threshold.fire += 1;
                }
                'W' => {
                    threshold.water += 1;
                }
                'E' => {
                    threshold.earth += 1;
                }
                'A' => {
                    threshold.air += 1;
                }
                _ => continue,
            }
        }
        threshold
    }

    pub fn zero() -> Self {
        Self {
            fire: 0,
            water: 0,
            earth: 0,
            air: 0,
        }
    }

    pub fn new(fire: u8, water: u8, earth: u8, air: u8) -> Self {
        Self {
            fire,
            water,
            earth,
            air,
        }
    }

    pub fn fire(amount: u8) -> Self {
        Self {
            fire: amount,
            ..Default::default()
        }
    }

    pub fn earth(amount: u8) -> Self {
        Self {
            earth: amount,
            ..Default::default()
        }
    }

    pub fn air(amount: u8) -> Self {
        Self {
            air: amount,
            ..Default::default()
        }
    }

    pub fn water(amount: u8) -> Self {
        Self {
            water: amount,
            ..Default::default()
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
        game_id: uuid::Uuid,
    },
    DrawCard {
        card_type: CardType,
        player_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    Sync {
        state: State,
    },
    CardSelected {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    CardPlayed {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        game_id: uuid::Uuid,
        targets: Target,
    },
    PrepareCardForPlay {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    Disconnect {
        player_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    EndTurn {
        player_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    ActionSelected {
        player_id: uuid::Uuid,
        action_idx: usize,
        game_id: uuid::Uuid,
    },
    SelectActionCancelled {
        player_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    AttackTarget {
        player_id: uuid::Uuid,
        attacker_id: uuid::Uuid,
        target_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
}

impl Message {
    pub fn get_game_id(&self) -> Option<uuid::Uuid> {
        match self {
            Message::Connect => None,
            Message::ConnectResponse { .. } => None,
            Message::MatchCreated { .. } => None,
            Message::Sync { .. } => None,
            Message::DrawCard { game_id, .. } => Some(game_id.clone()),
            Message::CardSelected { game_id, .. } => Some(game_id.clone()),
            Message::PrepareCardForPlay { game_id, .. } => Some(game_id.clone()),
            Message::CardPlayed { game_id, .. } => Some(game_id.clone()),
            Message::Disconnect { game_id, .. } => Some(game_id.clone()),
            Message::EndTurn { game_id, .. } => Some(game_id.clone()),
            Message::ActionSelected { game_id, .. } => Some(game_id.clone()),
            Message::SelectActionCancelled { game_id, .. } => Some(game_id.clone()),
            Message::AttackTarget { game_id, .. } => Some(game_id.clone()),
        }
    }
}
