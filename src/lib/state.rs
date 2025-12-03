use crate::{
    card::Card,
    deck::Deck,
    effect::Effect,
    game::{PlayerId, PlayerStatus, Resources},
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct State {
    pub cards: Vec<Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub resources: HashMap<PlayerId, Resources>,
    pub player_status: PlayerStatus,
    pub current_player: PlayerId,
    pub effects: VecDeque<Effect>,
}

impl State {
    pub fn new(cards: Vec<Box<dyn Card>>, decks: HashMap<PlayerId, Deck>) -> Self {
        State {
            cards,
            decks,
            resources: HashMap::new(),
            player_status: PlayerStatus::None,
            current_player: uuid::Uuid::nil(),
            effects: VecDeque::new(),
        }
    }
}
