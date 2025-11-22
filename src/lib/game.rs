use crate::{
    card::{avatar::Avatar, Card, CardBase, CardType, CardZone},
    deck::Deck,
    effect::{Action, Effect},
    networking::Message,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    None,
    WaitingForCardDraw { player_id: uuid::Uuid, count: u8 },
    SelectingCell { player_id: uuid::Uuid, cell_ids: Vec<u8> },
    WaitingForPlay { player_id: uuid::Uuid },
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
    pub turns_taken: u32,
    pub players: Vec<uuid::Uuid>,
    pub current_player: uuid::Uuid,
    pub next_player: uuid::Uuid,
    pub selected_cards: Vec<String>,
    pub cards: Vec<Card>,
    pub effects_queue: VecDeque<Effect>,
    pub resources: HashMap<uuid::Uuid, Resources>,
}

impl State {
    pub fn new(players: Vec<uuid::Uuid>) -> Self {
        State {
            phase: Phase::None,
            turns_taken: 0,
            current_player: uuid::Uuid::nil(),
            next_player: uuid::Uuid::nil(),
            players,
            selected_cards: vec![],
            cards: vec![],
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

    fn is_cell_empty(&self, cell_id: &u8) -> bool {
        !self
            .cards
            .iter()
            .filter(|c| c.get_type() != CardType::Avatar)
            .any(|card| match card.get_zone() {
                CardZone::Realm(id) => id == cell_id,
                _ => false,
            })
    }

    pub fn find_valid_cells_for_card(&self, card: &Card) -> Vec<u8> {
        let adjacent_cells = self.get_cells_adjacent_to_sites(&card.get_owner_id());
        match card.get_type() {
            CardType::Site => {
                let mut cells = vec![];
                for cell in &adjacent_cells {
                    if self.is_cell_empty(cell) {
                        cells.push(cell.clone());
                    }
                }

                cells
            }
            CardType::Spell => vec![],
            CardType::Avatar => vec![],
        }
    }

    pub fn has_played_site(&self, owner_id: &uuid::Uuid) -> bool {
        self.cards.iter().any(|card| {
            card.get_owner_id() == owner_id && card.is_site() && matches!(card.get_zone(), CardZone::Realm(_))
        })
    }

    pub fn is_player_one(&self, player_id: &uuid::Uuid) -> bool {
        if self.players.len() < 2 {
            return false;
        }
        &self.players[0] == player_id
    }

    pub fn get_cells_adjacent_to_sites(&self, owner_id: &uuid::Uuid) -> Vec<u8> {
        if !self.has_played_site(owner_id) {
            let starting_cell = if self.is_player_one(owner_id) { 3 } else { 18 };
            return vec![starting_cell];
        }

        let mut adjacent_cells = Vec::new();
        let site_cell_ids: Vec<u8> = self
            .cards
            .iter()
            .filter(|card| card.get_owner_id() == owner_id && card.is_site())
            .filter_map(|card| match card.get_zone() {
                CardZone::Realm(cell_id) => Some(*cell_id),
                _ => None,
            })
            .collect();

        for cell_id in site_cell_ids {
            let neighbors = Self::get_adjacent_cell_ids(cell_id);
            for neighbor in neighbors {
                if !adjacent_cells.contains(&neighbor) {
                    adjacent_cells.push(neighbor);
                }
            }
        }

        adjacent_cells
    }

    pub fn get_adjacent_cell_ids(cell_id: u8) -> Vec<u8> {
        let mut neighbors = Vec::new();
        let rows = 4;
        let cols = 5;
        let row = cell_id / cols;
        let col = cell_id % cols;

        if row > 0 {
            neighbors.push((row - 1) * cols + col);
        }
        if row < rows - 1 {
            neighbors.push((row + 1) * cols + col);
        }
        if col > 0 {
            neighbors.push(row * cols + (col - 1));
        }
        if col < cols - 1 {
            neighbors.push(row * cols + (col + 1));
        }

        neighbors
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
    pub addrs: HashMap<uuid::Uuid, SocketAddr>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(
        player1: uuid::Uuid,
        player2: uuid::Uuid,
        socket: Arc<UdpSocket>,
        addr1: SocketAddr,
        addr2: SocketAddr,
    ) -> Self {
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
            state: State::new(vec![player1, player2]),
            decks,
            socket,
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
        }
    }

    pub async fn process_message(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::CardPlayed {
                card_id,
                player_id,
                cell_id,
                ..
            } => self.card_played(&player_id, &card_id, cell_id).await?,
            Message::CardSelected { card_id, player_id, .. } => self.card_selected(&player_id, &card_id).await?,
            Message::EndTurn { player_id, .. } => self.end_turn(&player_id).await?,
            Message::DrawCard {
                card_type, player_id, ..
            } => self.draw_card_for_player(&player_id, card_type).await?,
            _ => {}
        }

        self.process_effects();
        self.send_sync().await?;
        Ok(())
    }

    pub fn process_effects(&mut self) {
        while let Some(effect) = self.state.effects_queue.pop_front() {
            effect.apply(&mut self.state);
        }
    }

    pub fn place_avatars(&mut self) -> anyhow::Result<()> {
        for player_id in &self.players {
            let deck = self.decks.get_mut(&player_id).unwrap();
            let mut avatar_card = Card::Avatar(deck.avatar.clone());
            let cell_id = if self.state.is_player_one(player_id) { 3 } else { 18 };
            avatar_card.set_zone(CardZone::Realm(cell_id));
            self.state.cards.push(avatar_card);
        }
        Ok(())
    }

    pub async fn send_sync(&self) -> anyhow::Result<()> {
        for player_id in &self.players {
            let addr = self.addrs.get(&player_id).unwrap();
            let message = Message::Sync {
                state: self.state.clone(),
            };
            self.send_message(&message, addr).await?;
        }
        Ok(())
    }

    async fn send_message(&self, message: &Message, addr: &SocketAddr) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(&message)?;
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    pub async fn send_to_player(&self, message: &Message, player_id: &uuid::Uuid) -> anyhow::Result<()> {
        let addr = self.addrs.get(player_id).unwrap();
        self.send_message(message, addr).await
    }

    pub async fn broadcast(&self, message: &Message) -> anyhow::Result<()> {
        for addr in self.addrs.values() {
            self.send_message(message, addr).await?;
        }
        Ok(())
    }

    pub async fn draw_initial_six(&mut self, player_id: &uuid::Uuid) -> anyhow::Result<()> {
        let deck = self.decks.get_mut(player_id).unwrap();
        deck.shuffle();

        self.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.draw_card_for_player(&player_id, CardType::Site).await?;
        self.draw_card_for_player(&player_id, CardType::Site).await?;
        self.draw_card_for_player(&player_id, CardType::Site).await?;
        Ok(())
    }

    pub async fn end_turn(&mut self, player_id: &uuid::Uuid) -> anyhow::Result<()> {
        assert!(self.state.is_players_turn(player_id));

        let resources = self.state.resources.get_mut(&self.state.current_player).unwrap();
        resources.mana = 0;

        self.state.turns_taken += 1;
        self.state.current_player = self
            .players
            .iter()
            .cycle()
            .skip(self.state.turns_taken as usize)
            .next()
            .unwrap()
            .clone();
        self.state.phase = Phase::WaitingForCardDraw {
            player_id: self.state.current_player.clone(),
            count: 1,
        };

        self.state
            .cards
            .iter()
            .filter(|card| card.get_owner_id() == &self.state.current_player)
            .filter(|card| matches!(card.get_zone(), CardZone::Realm(_)))
            .for_each(|card| {
                let effects = card.on_turn_start();
                self.state.effects_queue.extend(effects);
            });

        Ok(())
    }

    pub async fn card_selected(&mut self, player_id: &uuid::Uuid, card_id: &uuid::Uuid) -> anyhow::Result<()> {
        let state_clone = self.state.clone();
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id && card.get_owner_id() == player_id)
            .unwrap();
        let actions = card.on_select(&state_clone);
        for action in &actions {
            match action {
                Action::SelectCell { cell_ids } => {
                    self.state.phase = Phase::SelectingCell {
                        player_id: player_id.clone(),
                        cell_ids: cell_ids.clone(),
                    };
                }
            }
        }

        Ok(())
    }

    pub async fn card_played(
        &mut self,
        player_id: &uuid::Uuid,
        card_id: &uuid::Uuid,
        cell_id: u8,
    ) -> anyhow::Result<()> {
        assert!(cell_id >= 1 && cell_id <= 20);
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id && card.get_owner_id() == player_id)
            .unwrap();
        self.state.effects_queue.push_back(Effect::CardMovedToCell {
            card_id: card_id.clone(),
            cell_id,
        });
        self.state.effects_queue.extend(card.genesis());
        self.state.effects_queue.push_back(Effect::PhaseChanged {
            new_phase: Phase::WaitingForPlay {
                player_id: player_id.clone(),
            },
        });

        Ok(())
    }

    pub async fn draw_card_for_player(&mut self, player_id: &uuid::Uuid, card_type: CardType) -> anyhow::Result<()> {
        let deck = self.decks.get_mut(&player_id).unwrap();
        let card = match card_type {
            CardType::Site => deck.draw_site().map(|site| Card::Site(site)),
            CardType::Spell => deck.draw_spell().map(|spell| Card::Spell(spell)),
            CardType::Avatar => None,
        };

        if card.is_none() {
            return Ok(());
        }

        let mut card = card.unwrap();
        card.set_zone(CardZone::Hand);
        self.state.cards.push(card);

        match self.state.phase {
            Phase::WaitingForCardDraw { ref mut count, .. } => {
                *count -= 1;

                if *count == 0 {
                    self.state.effects_queue.push_back(Effect::PhaseChanged {
                        new_phase: Phase::WaitingForPlay {
                            player_id: player_id.clone(),
                        },
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
}
