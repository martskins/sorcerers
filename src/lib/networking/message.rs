use std::collections::HashMap;

use crate::{
    card::{Card, CardType, RenderableCard, Zone},
    deck::{Deck, precon},
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
pub enum PreconDeck {
    BetaFire,
    BetaAir,
}

impl PreconDeck {
    pub fn name(&self) -> &'static str {
        match self {
            PreconDeck::BetaFire => "Beta - Fire",
            PreconDeck::BetaAir => "Beta - Air",
        }
    }

    pub fn build(&self, player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
        match self {
            PreconDeck::BetaFire => precon::beta::fire(player_id),
            PreconDeck::BetaAir => precon::beta::air(player_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    LogEvent {
        id: uuid::Uuid,
        description: String,
        datetime: chrono::DateTime<chrono::Utc>,
    },
    ConnectResponse {
        player_id: PlayerId,
        available_decks: Vec<PreconDeck>,
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
    PickPath {
        prompt: String,
        player_id: PlayerId,
        paths: Vec<Vec<Zone>>,
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
            ServerMessage::LogEvent { .. } => uuid::Uuid::nil(),
            ServerMessage::PickDirection { player_id, .. } => player_id.clone(),
            ServerMessage::PickCard { player_id, .. } => player_id.clone(),
            ServerMessage::PickZone { player_id, .. } => player_id.clone(),
            ServerMessage::PickAction { player_id, .. } => player_id.clone(),
            ServerMessage::PickPath { player_id, .. } => player_id.clone(),
            ServerMessage::ConnectResponse { player_id, .. } => player_id.clone(),
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
    JoinQueue {
        player_id: PlayerId,
        deck: PreconDeck,
    },
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
    PickPath {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        path: Vec<Zone>,
    },
    PickZone {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        zone: Zone,
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
            ClientMessage::JoinQueue { .. } => uuid::Uuid::nil(),
            ClientMessage::PickCard { game_id, .. } => game_id.clone(),
            ClientMessage::PickAction { game_id, .. } => game_id.clone(),
            ClientMessage::EndTurn { game_id, .. } => game_id.clone(),
            ClientMessage::PickZone { game_id, .. } => game_id.clone(),
            ClientMessage::PickPath { game_id, .. } => game_id.clone(),
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
            ClientMessage::PickZone { player_id, .. } => player_id,
            ClientMessage::PickPath { player_id, .. } => player_id,
            ClientMessage::ClickCard { player_id, .. } => player_id,
            ClientMessage::DrawCard { player_id, .. } => player_id,
            ClientMessage::PickDirection { player_id, .. } => player_id,
            ClientMessage::JoinQueue { player_id, .. } => player_id,
        }
    }
}

impl ToMessage for ClientMessage {
    fn to_message(&self) -> Message {
        Message::ClientMessage(self.clone())
    }
}
