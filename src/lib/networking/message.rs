use std::collections::HashMap;

use crate::{
    card::{CardInfo, CardType},
    game::{PlayerId, PlayerStatus, Resources},
};
use serde::{Deserialize, Serialize};

pub trait ToMessage {
    fn to_message(&self) -> Message;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    ServerMessage(ServerMessage),
    ClientMessage(ClientMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ConnectResponse {
        player_id: PlayerId,
    },
    GameStarted {
        game_id: uuid::Uuid,
        player1: PlayerId,
        player2: PlayerId,
    },
    Sync {
        cards: Vec<CardInfo>,
        resources: HashMap<PlayerId, Resources>,
        player_status: PlayerStatus,
        current_player: PlayerId,
    },
}

impl ToMessage for ServerMessage {
    fn to_message(&self) -> Message {
        Message::ServerMessage(self.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect,
    DrawCard {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_type: CardType,
    },
    ClickCard {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    PlayCard {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    PickCard {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    PickSquare {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        square: u8,
    },
    PickAction {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        action_idx: usize,
    },
    EndTurn {
        game_id: uuid::Uuid,
        player_id: PlayerId,
    },
}

impl ClientMessage {
    pub fn game_id(&self) -> uuid::Uuid {
        match self {
            ClientMessage::Connect => uuid::Uuid::nil(),
            ClientMessage::PlayCard { game_id, .. } => game_id.clone(),
            ClientMessage::PickCard { game_id, .. } => game_id.clone(),
            ClientMessage::PickAction { game_id, .. } => game_id.clone(),
            ClientMessage::EndTurn { game_id, .. } => game_id.clone(),
            ClientMessage::PickSquare { game_id, .. } => game_id.clone(),
            ClientMessage::ClickCard { game_id, .. } => game_id.clone(),
            ClientMessage::DrawCard { game_id, .. } => game_id.clone(),
        }
    }

    pub fn player_id(&self) -> PlayerId {
        match self {
            ClientMessage::Connect => PlayerId::nil(),
            ClientMessage::PlayCard { player_id, .. } => player_id.clone(),
            ClientMessage::PickCard { player_id, .. } => player_id.clone(),
            ClientMessage::PickAction { player_id, .. } => player_id.clone(),
            ClientMessage::EndTurn { player_id, .. } => player_id.clone(),
            ClientMessage::PickSquare { player_id, .. } => player_id.clone(),
            ClientMessage::ClickCard { player_id, .. } => player_id.clone(),
            ClientMessage::DrawCard { player_id, .. } => player_id.clone(),
        }
    }
}

impl ToMessage for ClientMessage {
    fn to_message(&self) -> Message {
        Message::ClientMessage(self.clone())
    }
}
