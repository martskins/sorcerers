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
    pub fn snapshot(&self) -> State {
        State {
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            resources: self.resources.clone(),
            player_status: self.player_status.clone(),
            current_player: self.current_player,
            effects: self.effects.clone(),
        }
    }
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
