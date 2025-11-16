use crate::{
    card::{avatar::Avatar, Card, CardBase, CardZone},
    deck::Deck,
    effect::Effect,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    None,
    TurnStartPhase,
    WaitingForCardDrawPhase,
    WaitingForCellSelectionPhase,
    MainPhase,
    EndPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resources {
    pub fire_threshold: u8,
    pub water_threshold: u8,
    pub earth_threshold: u8,
    pub air_threshold: u8,
    pub mana: u8,
}

impl Resources {
    pub fn new() -> Self {
        Resources {
            fire_threshold: 0,
            water_threshold: 0,
            earth_threshold: 0,
            air_threshold: 0,
            mana: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub phase: Phase,
    pub turn_count: u32,
    pub current_player: uuid::Uuid,
    pub next_player: uuid::Uuid,
    pub selected_cards: Vec<String>,
    pub cards: Vec<Card>,
    pub cells: Vec<Cell>,
    pub effects_queue: VecDeque<Effect>,
    pub resources: HashMap<uuid::Uuid, Resources>,
}

impl State {
    pub fn new() -> Self {
        State {
            phase: Phase::None,
            turn_count: 0,
            current_player: uuid::Uuid::nil(),
            next_player: uuid::Uuid::nil(),
            selected_cards: vec![],
            cards: vec![],
            cells: vec![],
            effects_queue: VecDeque::new(),
            resources: HashMap::new(),
        }
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects_queue.push_back(effect);
    }

    pub fn is_players_turn(&self, player_id: &uuid::Uuid) -> bool {
        &self.current_player == player_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub id: u8,
    pub occupied_by: Vec<Card>,
}

pub struct Game {
    pub id: uuid::Uuid,
    pub players: Vec<uuid::Uuid>,
    pub decks: HashMap<uuid::Uuid, Deck>,
    pub state: State,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid) -> Self {
        let mut decks = HashMap::new();
        decks.insert(player1, Deck::test_deck(player1));
        let mut deck_two = Deck::test_deck(player2);
        deck_two.avatar = Avatar::Battlemage(CardBase {
            id: uuid::Uuid::new_v4(),
            owner_id: player2,
            zone: CardZone::Avatar,
        });
        decks.insert(player2, deck_two);

        Game {
            id: uuid::Uuid::new_v4(),
            players: vec![player1, player2],
            state: State::new(),
            decks,
        }
    }

    pub fn step(&mut self) {
        while let Some(effect) = self.state.effects_queue.pop_front() {
            effect.apply(&mut self.state);
        }
    }
}
