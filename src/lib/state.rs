use crate::{
    card::{Card, Zone},
    deck::Deck,
    effect::Effect,
    game::{PlayerId, Resources, Status},
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq, Clone)]
pub enum Phase {
    Main,
    PreEndTurn { player_id: PlayerId },
}

#[derive(Debug)]
pub struct State {
    pub turns: usize,
    pub cards: Vec<Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub resources: HashMap<PlayerId, Resources>,
    pub player_status: Status,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub current_player: PlayerId,
    pub effects: VecDeque<Effect>,
    pub player_one: PlayerId,
}

impl State {
    pub fn new(cards: Vec<Box<dyn Card>>, decks: HashMap<PlayerId, Deck>) -> Self {
        State {
            cards,
            decks,
            turns: 0,
            resources: HashMap::new(),
            player_status: Status::None,
            phase: Phase::Main,
            current_player: uuid::Uuid::nil(),
            waiting_for_input: false,
            effects: VecDeque::new(),
            player_one: uuid::Uuid::nil(),
        }
    }

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> Option<&mut Box<dyn Card>> {
        self.cards.iter_mut().find(|c| c.get_id() == card_id)
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> Option<&Box<dyn Card>> {
        self.cards.iter().find(|c| c.get_id() == card_id)
    }

    pub fn get_cards_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards.iter().filter(|c| c.get_zone() == zone).collect()
    }

    pub fn get_player_resources(&self, player_id: &PlayerId) -> &Resources {
        self.resources.get(player_id).unwrap()
    }

    pub fn snapshot(&self) -> State {
        State {
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: 0,
            resources: self.resources.clone(),
            player_status: self.player_status.clone(),
            phase: self.phase.clone(),
            current_player: self.current_player,
            waiting_for_input: self.waiting_for_input,
            effects: VecDeque::new(), // Effects are not needed in the snapshot
            player_one: self.player_one,
        }
    }
}
