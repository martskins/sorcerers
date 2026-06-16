use crate::{
    card::{Card, CardData, CardType},
    deck::{Deck, DeckList, precon::PreconDeck},
    game::{CardId, Direction, PlayerId, Resources, SoundEffect},
    zone::{Location, Zone},
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
pub struct OngoingEffectData {
    pub source_card_id: Option<CardId>,
    pub source_name: Option<String>,
    pub description: String,
    pub timestamp: u64,
    pub active: bool,
    pub affected_card_ids: Vec<CardId>,
    pub affected_zones: Vec<Zone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectDebugData {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    RevealCards {
        player_id: PlayerId,
        cards: Vec<CardId>,
        prompt: String,
        action: Option<String>,
    },
    DistributeDamage {
        player_id: PlayerId,
        attacker: CardId,
        defenders: Vec<CardId>,
        damage: u16,
    },
    PlaySoundEffect {
        player_id: Option<PlayerId>,
        sound_effect: SoundEffect,
    },
    ProjectileFired {
        player_id: PlayerId,
        shooter: CardId,
        path: Vec<Location>,
        direction: Direction,
        ranged_strike: bool,
    },
    PlayerDisconnected {
        player_id: PlayerId,
    },
    GameOver {
        player_id: PlayerId,
        winner_id: PlayerId,
        winner_name: String,
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
        card_id: CardId,
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
        turn_player: PlayerId,
        #[serde(default)]
        stepped_effects: bool,
        #[serde(default)]
        effect_queue: Vec<EffectDebugData>,
    },
    PickCards {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        cards: Vec<CardId>,
        preview: bool,
    },
    PickCard {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        cards: Vec<CardId>,
        pickable_cards: Vec<CardId>,
        preview: bool,
    },
    PickAmount {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        min_amount: u8,
        max_amount: u8,
    },
    PickAction {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        actions: Vec<String>,
        anchor_on_cursor: bool,
    },
    PickPath {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        paths: Vec<Vec<Location>>,
    },
    PickLocation {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        locations: Vec<Location>,
    },
    PlayableLocations {
        player_id: PlayerId,
        card_id: CardId,
        locations: Vec<Location>,
    },
    AuraAreOfEffect {
        player_id: PlayerId,
        card_id: CardId,
        locations: Vec<Location>,
    },
    OngoingEffects {
        player_id: PlayerId,
        effects: Vec<OngoingEffectData>,
    },
    PickLocationGroup {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        groups: Vec<Vec<Location>>,
    },
    PickDirection {
        prompt: String,
        source_card_id: Option<CardId>,
        player_id: PlayerId,
        directions: Vec<Direction>,
    },
    ForceSync {
        player_id: PlayerId,
        cards: Vec<CardData>,
        resources: HashMap<PlayerId, Resources>,
        current_player: PlayerId,
        turn_player: PlayerId,
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
            ServerMessage::ProjectileFired { player_id, .. } => *player_id,
            ServerMessage::Resume { player_id, .. } => *player_id,
            ServerMessage::Wait { player_id, .. } => *player_id,
            ServerMessage::PickDirection { player_id, .. } => *player_id,
            ServerMessage::PickCard { player_id, .. } => *player_id,
            ServerMessage::PickLocation { player_id, .. } => *player_id,
            ServerMessage::PlayableLocations { player_id, .. } => *player_id,
            ServerMessage::AuraAreOfEffect { player_id, .. } => *player_id,
            ServerMessage::OngoingEffects { player_id, .. } => *player_id,
            ServerMessage::PickLocationGroup { player_id, .. } => *player_id,
            ServerMessage::PickAction { player_id, .. } => *player_id,
            ServerMessage::PickPath { player_id, .. } => *player_id,
            ServerMessage::ConnectResponse { player_id, .. } => *player_id,
            ServerMessage::GameStarted { .. } => uuid::Uuid::nil(),
            ServerMessage::Sync { .. } => uuid::Uuid::nil(),
            ServerMessage::ForceSync { player_id, .. } => *player_id,
            ServerMessage::PlayerDisconnected { player_id } => *player_id,
            ServerMessage::GameOver { player_id, .. } => *player_id,
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
        damage_assignment: HashMap<CardId, u16>,
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
        card_id: CardId,
    },
    RequestPlayableLocations {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: CardId,
    },
    RequestAuraAreaOfEffect {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: CardId,
    },
    RequestOngoingEffects {
        game_id: uuid::Uuid,
        player_id: PlayerId,
    },
    PlayCardAtLocation {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: CardId,
        location: Location,
    },
    PickDirection {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        direction: Direction,
    },
    PickCards {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_ids: Vec<CardId>,
    },
    PickCard {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        card_id: CardId,
    },
    PickPath {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        path: Vec<Location>,
    },
    PickLocationGroup {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        group_idx: usize,
    },
    PickLocation {
        game_id: uuid::Uuid,
        player_id: PlayerId,
        location: Location,
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
    ToggleSteppedEffects {
        game_id: uuid::Uuid,
        player_id: PlayerId,
    },
    StepNextEffect {
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
            ClientMessage::ToggleSteppedEffects { game_id, .. } => *game_id,
            ClientMessage::StepNextEffect { game_id, .. } => *game_id,
            ClientMessage::PickLocation { game_id, .. } => *game_id,
            ClientMessage::PickLocationGroup { game_id, .. } => *game_id,
            ClientMessage::PickPath { game_id, .. } => *game_id,
            ClientMessage::ClickCard { game_id, .. } => *game_id,
            ClientMessage::RequestPlayableLocations { game_id, .. } => *game_id,
            ClientMessage::RequestAuraAreaOfEffect { game_id, .. } => *game_id,
            ClientMessage::RequestOngoingEffects { game_id, .. } => *game_id,
            ClientMessage::PlayCardAtLocation { game_id, .. } => *game_id,
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
            ClientMessage::ToggleSteppedEffects { player_id, .. } => player_id,
            ClientMessage::StepNextEffect { player_id, .. } => player_id,
            ClientMessage::PickLocation { player_id, .. } => player_id,
            ClientMessage::PickLocationGroup { player_id, .. } => player_id,
            ClientMessage::PickPath { player_id, .. } => player_id,
            ClientMessage::ClickCard { player_id, .. } => player_id,
            ClientMessage::RequestPlayableLocations { player_id, .. } => player_id,
            ClientMessage::RequestAuraAreaOfEffect { player_id, .. } => player_id,
            ClientMessage::RequestOngoingEffects { player_id, .. } => player_id,
            ClientMessage::PlayCardAtLocation { player_id, .. } => player_id,
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
