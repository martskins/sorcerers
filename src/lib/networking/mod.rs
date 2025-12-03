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
    SelectCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    PlayCard {
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
    TriggerAction {
        player_id: uuid::Uuid,
        action_idx: usize,
        game_id: uuid::Uuid,
    },
    CancelSelectAction {
        player_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    AttackTarget {
        player_id: uuid::Uuid,
        attacker_id: uuid::Uuid,
        target_id: uuid::Uuid,
        game_id: uuid::Uuid,
    },
    MoveCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        square: u8,
        game_id: uuid::Uuid,
    },
    SummonMinion {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        game_id: uuid::Uuid,
        square: u8,
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
            Message::SelectCard { game_id, .. } => Some(game_id.clone()),
            Message::PrepareCardForPlay { game_id, .. } => Some(game_id.clone()),
            Message::PlayCard { game_id, .. } => Some(game_id.clone()),
            Message::Disconnect { game_id, .. } => Some(game_id.clone()),
            Message::EndTurn { game_id, .. } => Some(game_id.clone()),
            Message::TriggerAction { game_id, .. } => Some(game_id.clone()),
            Message::CancelSelectAction { game_id, .. } => Some(game_id.clone()),
            Message::AttackTarget { game_id, .. } => Some(game_id.clone()),
            Message::MoveCard { game_id, .. } => Some(game_id.clone()),
            Message::SummonMinion { game_id, .. } => Some(game_id.clone()),
        }
    }
}
