use std::collections::HashMap;

use crate::{card::Card, deck::Deck};

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
}

impl State {
    pub fn zero() -> Self {
        State {
            phase: Phase::None,
            turn_count: 0,
            current_player: uuid::Uuid::nil(),
            next_player: uuid::Uuid::nil(),
            selected_cards: vec![],
        }
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
    pub cards: Vec<Card>,
    pub cells: Vec<Cell>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid) -> Self {
        let mut decks = HashMap::new();
        decks.insert(player1, Deck::test_deck(player1));
        let mut deck_two = Deck::test_deck(player2);
        deck_two.avatar.name = "Sorcerer".to_string();
        decks.insert(player2, deck_two);

        Game {
            id: uuid::Uuid::new_v4(),
            players: vec![player1, player2],
            state: State::zero(),
            cards: vec![],
            decks,
            cells: vec![],
        }
    }
}
