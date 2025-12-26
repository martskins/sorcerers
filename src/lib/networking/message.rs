use std::collections::HashMap;

use crate::{
    card::{CardType, RenderableCard, Zone},
    game::{Direction, PlayerId, Resources},
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
        cards: Vec<RenderableCard>,
    },
    Sync {
        cards: Vec<RenderableCard>,
        resources: HashMap<PlayerId, Resources>,
        current_player: PlayerId,
    },
    PickCard {
        prompt: String,
        player_id: PlayerId,
        cards: Vec<uuid::Uuid>,
        preview: bool,
    },
    PickAction {
        prompt: String,
        player_id: PlayerId,
        actions: Vec<String>,
    },
    PickZone {
        prompt: String,
        player_id: PlayerId,
        zones: Vec<Zone>,
    },
    PickDirection {
        prompt: String,
        player_id: PlayerId,
        directions: Vec<Direction>,
    },
}

impl ServerMessage {
    pub fn player_id(&self) -> uuid::Uuid {
        match self {
            ServerMessage::PickDirection { player_id, .. } => player_id.clone(),
            ServerMessage::PickCard { player_id, .. } => player_id.clone(),
            ServerMessage::PickZone { player_id, .. } => player_id.clone(),
            ServerMessage::PickAction { player_id, .. } => player_id.clone(),
            ServerMessage::ConnectResponse { player_id } => player_id.clone(),
            ServerMessage::GameStarted { .. } => uuid::Uuid::nil(),
            ServerMessage::Sync { .. } => uuid::Uuid::nil(),
        }
    }
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
    PickDirection {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        direction: Direction,
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

const NIL: uuid::Uuid = PlayerId::nil();

impl ClientMessage {
    pub fn game_id(&self) -> uuid::Uuid {
        match self {
            ClientMessage::Connect => uuid::Uuid::nil(),
            ClientMessage::PickCard { game_id, .. } => game_id.clone(),
            ClientMessage::PickAction { game_id, .. } => game_id.clone(),
            ClientMessage::EndTurn { game_id, .. } => game_id.clone(),
            ClientMessage::PickSquare { game_id, .. } => game_id.clone(),
            ClientMessage::ClickCard { game_id, .. } => game_id.clone(),
            ClientMessage::DrawCard { game_id, .. } => game_id.clone(),
            ClientMessage::PickDirection { game_id, .. } => game_id.clone(),
        }
    }

    pub fn player_id(&self) -> &PlayerId {
        match self {
            ClientMessage::Connect => &NIL,
            ClientMessage::PickCard { player_id, .. } => player_id,
            ClientMessage::PickAction { player_id, .. } => player_id,
            ClientMessage::EndTurn { player_id, .. } => player_id,
            ClientMessage::PickSquare { player_id, .. } => player_id,
            ClientMessage::ClickCard { player_id, .. } => player_id,
            ClientMessage::DrawCard { player_id, .. } => player_id,
            ClientMessage::PickDirection { player_id, .. } => player_id,
        }
    }
}

impl ToMessage for ClientMessage {
    fn to_message(&self) -> Message {
        Message::ClientMessage(self.clone())
    }
}
