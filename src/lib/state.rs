use async_channel::{Receiver, Sender};

use crate::{
    card::{Card, Zone},
    deck::Deck,
    effect::Effect,
    game::{InputStatus, PlayerId, Resources},
    networking::message::{ClientMessage, ServerMessage},
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq, Clone)]
pub enum Phase {
    Main,
    PreEndTurn { player_id: PlayerId },
}

#[derive(Debug)]
pub struct EffectLog {
    pub queue: VecDeque<Effect>,
    pub idx: usize,
    pub len: usize,
}

impl EffectLog {
    pub fn new() -> Self {
        EffectLog {
            queue: VecDeque::new(),
            idx: 0,
            len: 0,
        }
    }

    pub fn push_back(&mut self, effect: Effect) {
        self.len += 1;
        self.queue.push_back(effect);
    }

    pub fn push_front(&mut self, effect: Effect) {
        self.len += 1;
        self.queue.push_front(effect);
    }

    pub fn extend(&mut self, effects: Vec<Effect>) {
        self.len += effects.len();
        self.queue.extend(effects);
    }

    pub fn pop_front(&mut self) -> Option<Effect> {
        self.idx = self.idx.saturating_add(1);
        self.len = self.len.saturating_sub(1);
        self.queue.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<Effect> {
        self.len = self.len.saturating_sub(1);
        self.queue.pop_back()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
}

#[derive(Debug)]
pub struct State {
    pub game_id: uuid::Uuid,
    pub players: Vec<Player>,
    pub turns: usize,
    pub cards: Vec<Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub resources: HashMap<PlayerId, Resources>,
    pub input_status: InputStatus,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub current_player: PlayerId,
    pub effects: EffectLog,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
}

impl State {
    pub fn new(
        game_id: uuid::Uuid,
        players: Vec<Player>,
        cards: Vec<Box<dyn Card>>,
        decks: HashMap<PlayerId, Deck>,
        server_tx: Sender<ServerMessage>,
        client_rx: Receiver<ClientMessage>,
    ) -> Self {
        State {
            game_id,
            players,
            cards,
            decks,
            turns: 0,
            resources: HashMap::new(),
            input_status: InputStatus::None,
            phase: Phase::Main,
            current_player: uuid::Uuid::nil(),
            waiting_for_input: false,
            effects: EffectLog::new(),
            player_one: uuid::Uuid::nil(),
            server_tx,
            client_rx,
        }
    }

    pub fn get_receiver(&self) -> Receiver<ClientMessage> {
        self.client_rx.clone()
    }

    pub fn get_sender(&self) -> Sender<ServerMessage> {
        self.server_tx.clone()
    }

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> Option<&mut Box<dyn Card>> {
        self.cards.iter_mut().find(|c| c.get_id() == card_id)
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> Option<&Box<dyn Card>> {
        self.cards.iter().find(|c| c.get_id() == card_id)
    }

    pub fn get_minions_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards
            .iter()
            .filter(|c| c.get_zone() == zone)
            .filter(|c| c.is_minion())
            .collect()
    }

    pub fn get_units_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards
            .iter()
            .filter(|c| c.get_zone() == zone)
            .filter(|c| c.is_unit())
            .collect()
    }

    pub fn get_cards_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards.iter().filter(|c| c.get_zone() == zone).collect()
    }

    pub fn get_player_resources(&self, player_id: &PlayerId) -> &Resources {
        self.resources.get(player_id).unwrap()
    }

    pub fn snapshot(&self) -> State {
        State {
            game_id: self.game_id.clone(),
            players: self.players.clone(),
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: 0,
            resources: self.resources.clone(),
            input_status: self.input_status.clone(),
            phase: self.phase.clone(),
            current_player: self.current_player,
            waiting_for_input: self.waiting_for_input,
            effects: EffectLog::new(),
            player_one: self.player_one,
            server_tx: self.server_tx.clone(),
            client_rx: self.client_rx.clone(),
        }
    }
}
