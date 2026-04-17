use crate::{
    card::{Card, CardData, CardType, Zone},
    deck::{Deck, DeckList, precon::PreconDeck},
    game::{Direction, PlayerId, Resources, SoundEffect},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait ToMessage {
    fn to_message(&self) -> Message;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    ServerMessage(ServerMessage),
    ClientMessage(ClientMessage),
}

/// A deck choice: either a preconstructed deck or a custom saved deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeckChoice {
    Precon(PreconDeck),
    Custom(DeckList),
}

impl DeckChoice {
    pub fn name(&self) -> String {
        match self {
            DeckChoice::Precon(p) => p.name().to_string(),
            DeckChoice::Custom(d) => d.name.clone(),
        }
    }

    pub fn build(&self, player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
        match self {
            DeckChoice::Precon(p) => p.build(player_id),
            DeckChoice::Custom(d) => d.build(player_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    RevealCards {
        player_id: PlayerId,
        cards: Vec<uuid::Uuid>,
        prompt: String,
        action: Option<String>,
    },
    DistributeDamage {
        player_id: PlayerId,
        attacker: uuid::Uuid,
        defenders: Vec<uuid::Uuid>,
        damage: u16,
    },
    PlaySoundEffect {
        player_id: Option<PlayerId>,
        sound_effect: SoundEffect,
    },
    PlayerDisconnected {
        player_id: PlayerId,
    },
    Resume {
        player_id: PlayerId,
    },
    Wait {
        player_id: PlayerId,
        prompt: String,
    },
    LogEvent {
        id: uuid::Uuid,
        description: String,
        datetime: chrono::DateTime<chrono::Utc>,
    },
    CardPlayed {
        card_id: uuid::Uuid,
        description: String,
    },
    ConnectResponse {
        player_id: PlayerId,
        available_decks: Vec<PreconDeck>,
    },
    GameStarted {
        game_id: uuid::Uuid,
        player1: PlayerId,
        player2: PlayerId,
        cards: Vec<CardData>,
    },
    Sync {
        cards: Vec<CardData>,
        resources: HashMap<PlayerId, Resources>,
        health: HashMap<PlayerId, u16>,
        current_player: PlayerId,
    },
    PickCards {
        prompt: String,
        player_id: PlayerId,
        cards: Vec<uuid::Uuid>,
        preview: bool,
    },
    PickCard {
        prompt: String,
        player_id: PlayerId,
        cards: Vec<uuid::Uuid>,
        pickable_cards: Vec<uuid::Uuid>,
        preview: bool,
    },
    PickAmount {
        prompt: String,
        player_id: PlayerId,
        min_amount: u8,
        max_amount: u8,
    },
    PickAction {
        prompt: String,
        player_id: PlayerId,
        actions: Vec<String>,
        anchor_on_cursor: bool,
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
    PickZoneGroup {
        prompt: String,
        player_id: PlayerId,
        groups: Vec<Vec<Zone>>,
    },
    PickDirection {
        prompt: String,
        player_id: PlayerId,
        directions: Vec<Direction>,
    },
    ForceSync {
        player_id: PlayerId,
        cards: Vec<CardData>,
        resources: HashMap<PlayerId, Resources>,
        current_player: PlayerId,
        health: HashMap<PlayerId, u16>,
    },
    MulligansEnded,
}

impl ServerMessage {
    pub fn player_id(&self) -> uuid::Uuid {
        match self {
            ServerMessage::MulligansEnded => uuid::Uuid::nil(),
            ServerMessage::LogEvent { .. } => uuid::Uuid::nil(),
            ServerMessage::CardPlayed { .. } => uuid::Uuid::nil(),
            ServerMessage::PlaySoundEffect { player_id, .. } => player_id.unwrap_or_default(),
            ServerMessage::Resume { player_id, .. } => *player_id,
            ServerMessage::Wait { player_id, .. } => *player_id,
            ServerMessage::PickDirection { player_id, .. } => *player_id,
            ServerMessage::PickCard { player_id, .. } => *player_id,
            ServerMessage::PickZone { player_id, .. } => *player_id,
            ServerMessage::PickZoneGroup { player_id, .. } => *player_id,
            ServerMessage::PickAction { player_id, .. } => *player_id,
            ServerMessage::PickPath { player_id, .. } => *player_id,
            ServerMessage::ConnectResponse { player_id, .. } => *player_id,
            ServerMessage::GameStarted { .. } => uuid::Uuid::nil(),
            ServerMessage::Sync { .. } => uuid::Uuid::nil(),
            ServerMessage::ForceSync { player_id, .. } => *player_id,
            ServerMessage::PlayerDisconnected { player_id } => *player_id,
            ServerMessage::PickCards { player_id, .. } => *player_id,
            ServerMessage::RevealCards { player_id, .. } => *player_id,
            ServerMessage::DistributeDamage { player_id, .. } => *player_id,
            ServerMessage::PickAmount { player_id, .. } => *player_id,
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
    Disconnect,
    ResolveAction {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        take_action: bool,
    },
    ResolveCombat {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        damage_assignment: HashMap<uuid::Uuid, u16>,
    },
    PlayerDisconnected {
        game_id: uuid::Uuid,
        player_id: PlayerId,
    },
    JoinQueue {
        player_name: String,
        player_id: PlayerId,
        deck: DeckChoice,
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
    PickCards {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_ids: Vec<uuid::Uuid>,
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
    PickZoneGroup {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        group_idx: usize,
    },
    PickZone {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        zone: Zone,
    },
    PickAmount {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        amount: u8,
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
            ClientMessage::Disconnect => uuid::Uuid::nil(),
            ClientMessage::JoinQueue { .. } => uuid::Uuid::nil(),
            ClientMessage::PlayerDisconnected { game_id, .. } => *game_id,
            ClientMessage::PickCard { game_id, .. } => *game_id,
            ClientMessage::PickAction { game_id, .. } => *game_id,
            ClientMessage::EndTurn { game_id, .. } => *game_id,
            ClientMessage::PickZone { game_id, .. } => *game_id,
            ClientMessage::PickZoneGroup { game_id, .. } => *game_id,
            ClientMessage::PickPath { game_id, .. } => *game_id,
            ClientMessage::ClickCard { game_id, .. } => *game_id,
            ClientMessage::DrawCard { game_id, .. } => *game_id,
            ClientMessage::PickDirection { game_id, .. } => *game_id,
            ClientMessage::PickCards { game_id, .. } => *game_id,
            ClientMessage::ResolveCombat { game_id, .. } => *game_id,
            ClientMessage::ResolveAction { game_id, .. } => *game_id,
            ClientMessage::PickAmount { game_id, .. } => *game_id,
        }
    }

    pub fn player_id(&self) -> &PlayerId {
        match self {
            ClientMessage::Connect => &NIL,
            ClientMessage::Disconnect => &NIL,
            ClientMessage::PlayerDisconnected { player_id, .. } => player_id,
            ClientMessage::PickCard { player_id, .. } => player_id,
            ClientMessage::PickAction { player_id, .. } => player_id,
            ClientMessage::EndTurn { player_id, .. } => player_id,
            ClientMessage::PickZone { player_id, .. } => player_id,
            ClientMessage::PickZoneGroup { player_id, .. } => player_id,
            ClientMessage::PickPath { player_id, .. } => player_id,
            ClientMessage::ClickCard { player_id, .. } => player_id,
            ClientMessage::DrawCard { player_id, .. } => player_id,
            ClientMessage::PickDirection { player_id, .. } => player_id,
            ClientMessage::JoinQueue { player_id, .. } => player_id,
            ClientMessage::PickCards { player_id, .. } => player_id,
            ClientMessage::ResolveCombat { player_id, .. } => player_id,
            ClientMessage::ResolveAction { player_id, .. } => player_id,
            ClientMessage::PickAmount { player_id, .. } => player_id,
        }
    }
}

impl ToMessage for ClientMessage {
    fn to_message(&self) -> Message {
        Message::ClientMessage(self.clone())
    }
}
