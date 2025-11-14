use std::collections::{HashMap, VecDeque};

use crate::{
    card::{avatar::Avatar, Card, CardBase, CardZone},
    deck::Deck,
    effect::Effect,
    networking::Thresholds,
};

pub enum Phase {
    None,
    TurnStartPhase,
    WaitingForCardDrawPhase,
    WaitingForCellSelectionPhase,
    MainPhase,
    EndPhase,
}

pub struct State {
    pub phase: Phase,
    pub turn_count: u32,
    pub current_player: uuid::Uuid,
    pub next_player: uuid::Uuid,
    pub selected_cards: Vec<String>,
    pub cards: Vec<Card>,
    pub cells: Vec<Cell>,
    pub effects_queue: VecDeque<Effect>,
    pub player_mana: HashMap<uuid::Uuid, u8>,
    pub player_thresholds: HashMap<uuid::Uuid, Thresholds>,
}

impl State {
    pub fn zero() -> Self {
        State {
            phase: Phase::None,
            turn_count: 0,
            current_player: uuid::Uuid::nil(),
            next_player: uuid::Uuid::nil(),
            selected_cards: vec![],
            cards: vec![],
            cells: vec![],
            effects_queue: VecDeque::new(),
            player_mana: HashMap::new(),
            player_thresholds: HashMap::new(),
        }
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects_queue.push_back(effect);
    }
}

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
            state: State::zero(),
            decks,
        }
    }

    pub fn step(&mut self) {
        println!("Stepping game {}", self.id);
        while let Some(effect) = self.state.effects_queue.pop_front() {
            println!("Applying effect in game {}", self.id);
            effect.apply(&mut self.state);
        }
    }
}
