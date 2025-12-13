use crate::{
    card::{Card, Zone},
    deck::Deck,
    effect::Effect,
    game::{are_adjacent, get_adjacent_squares, get_nearby_squares, PlayerId, PlayerStatus, Resources},
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct State {
    pub turns: usize,
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
            turns: 0,
            resources: HashMap::new(),
            player_status: PlayerStatus::None,
            current_player: uuid::Uuid::nil(),
            effects: VecDeque::new(),
        }
    }

    pub fn snapshot(&self) -> State {
        State {
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: 0,
            resources: self.resources.clone(),
            player_status: self.player_status.clone(),
            current_player: self.current_player,
            effects: self.effects.clone(),
        }
    }

    pub fn get_valid_site_squares(&self, player_id: &PlayerId) -> Vec<u8> {
        let has_played_site = self
            .cards
            .iter()
            .any(|c| c.get_owner_id() == player_id && c.is_site() && matches!(c.get_zone(), Zone::Realm(_)));
        if !has_played_site {
            let avatar = self
                .cards
                .iter()
                .find(|c| c.get_owner_id() == player_id && c.is_avatar())
                .unwrap();
            match avatar.get_zone() {
                Zone::Realm(s) => return vec![s],
                _ => panic!("Avatar not in realm"),
            }
        }

        let occupied_squares: Vec<u8> = self
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == player_id)
            .filter(|c| c.is_site() || c.is_avatar())
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .flat_map(|c| match c.get_zone() {
                Zone::Realm(s) => vec![s],
                _ => vec![],
            })
            .collect();

        dbg!(&occupied_squares)
            .iter()
            .flat_map(|c| get_adjacent_squares(*c))
            .filter(|c| !occupied_squares.contains(c))
            .collect()
    }
}
