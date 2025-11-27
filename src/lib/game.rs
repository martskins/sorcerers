use crate::{
    card::{avatar::Avatar, spell::Spell, Card, CardBase, CardType, CardZone, Target},
    deck::Deck,
    effect::{Action, Effect, GameAction, PlayerAction},
    networking::{Message, Socket},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    None,
    WaitingForCardDraw {
        player_id: uuid::Uuid,
        count: u8,
        types: Vec<CardType>,
    },
    SelectingCell {
        player_id: uuid::Uuid,
        cell_ids: Vec<u8>,
        after_select: Option<Action>,
    },
    SelectingAction {
        player_id: uuid::Uuid,
        actions: Vec<Action>,
    },
    WaitingForPlay {
        player_id: uuid::Uuid,
    },
    SelectingCard {
        player_id: uuid::Uuid,
        card_ids: Vec<uuid::Uuid>,
        amount: u8,
        after_select: Option<Action>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resources {
    pub fire_threshold: u8,
    pub water_threshold: u8,
    pub earth_threshold: u8,
    pub air_threshold: u8,
    pub mana: u8,
    pub health: u8,
}

impl Resources {
    pub fn new() -> Self {
        Resources {
            fire_threshold: 0,
            water_threshold: 0,
            earth_threshold: 0,
            air_threshold: 0,
            mana: 0,
            health: 20,
        }
    }

    pub fn has_enough_for_spell(&self, card: &Spell) -> bool {
        let mana = card.get_mana_cost();
        let thresholds = card.get_required_threshold();

        self.mana >= mana
            && self.fire_threshold >= thresholds.fire
            && self.water_threshold >= thresholds.water
            && self.earth_threshold >= thresholds.earth
            && self.air_threshold >= thresholds.air
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub phase: Phase,
    pub turns_taken: u32,
    pub players: Vec<uuid::Uuid>,
    pub current_player: uuid::Uuid,
    pub cards: Vec<Card>,
    pub effects: VecDeque<Effect>,
    pub resources: HashMap<uuid::Uuid, Resources>,
    pub actions: HashMap<uuid::Uuid, Vec<Action>>,
    pub current_target: Option<Target>,
    pub targeting: u8,
}

impl State {
    pub fn new(players: Vec<uuid::Uuid>) -> Self {
        State {
            phase: Phase::None,
            turns_taken: 0,
            current_player: uuid::Uuid::nil(),
            players,
            cards: vec![],
            effects: VecDeque::new(),
            resources: HashMap::new(),
            actions: HashMap::new(),
            current_target: None,
            targeting: 0,
        }
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push_back(effect);
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
    pub addrs: HashMap<uuid::Uuid, Socket>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid, socket: Arc<UdpSocket>, addr1: Socket, addr2: Socket) -> Self {
        let mut decks = HashMap::new();
        let deck_one = Deck::test_deck(player1);

        let mut deck_two = Deck::test_deck(player2);
        deck_two.avatar = Avatar::Battlemage(CardBase {
            id: uuid::Uuid::new_v4(),
            owner_id: player2,
            zone: CardZone::Realm(18),
            tapped: false,
        });

        let state = State::new(vec![player1, player2]);
        decks.insert(player1, deck_one);
        decks.insert(player2, deck_two);

        Game {
            id: uuid::Uuid::new_v4(),
            players: vec![player1, player2],
            state,
            decks,
            socket,
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
        }
    }

    pub async fn process_message(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::PrepareCardForPlay { card_id, player_id, .. } => {
                self.prepare_card_for_play(&player_id, &card_id).await?
            }
            Message::CardPlayed {
                card_id,
                player_id,
                targets,
                ..
            } => self.card_played(&player_id, &card_id, targets).await?,
            Message::CardSelected { card_id, player_id, .. } => self.card_selected(&player_id, &card_id).await?,
            Message::EndTurn { player_id, .. } => self.end_turn(&player_id).await?,
            Message::DrawCard {
                card_type, player_id, ..
            } => self.draw_card_for_player(&player_id, card_type).await?,
            Message::ActionSelected {
                player_id, action_idx, ..
            } => self.trigger_action(&player_id, action_idx).await?,
            Message::SelectActionCancelled { player_id, .. } => {
                self.state.phase = Phase::WaitingForPlay { player_id };
            }
            _ => {}
        }

        self.process_effects();
        self.send_sync().await?;
        Ok(())
    }

    async fn trigger_action(&mut self, player_id: &uuid::Uuid, action_idx: usize) -> anyhow::Result<()> {
        let actions = self.state.actions.get(player_id).unwrap();
        if action_idx >= actions.len() {
            return Ok(());
        }

        match &actions[action_idx] {
            Action::PlayerAction(PlayerAction::DrawSite { after_select }) => {
                self.state.effects.extend(after_select.clone());
                self.draw_card_for_player(player_id, CardType::Site).await?;
                self.state.effects.push_back(Effect::ChangePhase {
                    new_phase: Phase::WaitingForPlay {
                        player_id: player_id.clone(),
                    },
                });
            }
            Action::PlayerAction(PlayerAction::PlaySite { after_select }) => {
                self.state.effects.extend(after_select.clone());
                let site_ids = self
                    .state
                    .cards
                    .iter()
                    .filter(|card| card.get_owner_id() == player_id && card.is_site())
                    .filter(|card| matches!(card.get_zone(), CardZone::Hand))
                    .map(|card| card.get_id())
                    .cloned()
                    .collect();
                self.state.effects.push_back(Effect::ChangePhase {
                    new_phase: Phase::SelectingCard {
                        player_id: player_id.clone(),
                        card_ids: site_ids,
                        amount: 1,
                        after_select: Some(Action::GameAction(GameAction::PlaySelectedCard)),
                    },
                });
            }
            _ => {}
        }

        Ok(())
    }

    pub fn process_effects(&mut self) {
        while let Some(effect) = self.state.effects.pop_front() {
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

    async fn send_message(&self, message: &Message, addr: &Socket) -> anyhow::Result<()> {
        match addr {
            Socket::SocketAddr(addr) => {
                let bytes = rmp_serde::to_vec(&message)?;
                self.socket.send_to(&bytes, addr).await?;
            }
            Socket::Noop => {}
        }

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
            types: vec![CardType::Site, CardType::Spell],
        };

        let state = self.state.clone();
        self.state
            .cards
            .iter()
            .filter(|card| card.get_owner_id() == &self.state.current_player)
            .filter(|card| matches!(card.get_zone(), CardZone::Realm(_)))
            .for_each(|card| {
                let effects = card.on_turn_start(&state);
                self.state.effects.extend(effects);
            });

        Ok(())
    }

    pub async fn card_selected(&mut self, player_id: &uuid::Uuid, card_id: &uuid::Uuid) -> anyhow::Result<()> {
        let state_clone = self.state.clone();
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id && card.get_owner_id() == player_id);
        if card.is_none() {
            return Ok(());
        }

        if self.state.targeting > 0 {
            self.state.current_target = Some(Target::Cards(vec![card_id.clone()]));
            self.state.targeting -= 1;
        }

        let effects = card.unwrap().on_select(&state_clone);
        self.state.effects.extend(effects);
        Ok(())
    }

    pub async fn prepare_card_for_play(&mut self, player_id: &uuid::Uuid, card_id: &uuid::Uuid) -> anyhow::Result<()> {
        let state = self.state.clone();
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id && card.get_owner_id() == player_id);
        if card.is_none() {
            return Ok(());
        }

        let effects = card.unwrap().on_prepare(&state);
        self.state.effects.extend(effects);
        Ok(())
    }

    pub async fn card_played(
        &mut self,
        player_id: &uuid::Uuid,
        card_id: &uuid::Uuid,
        target: Target,
    ) -> anyhow::Result<()> {
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id)
            .cloned()
            .unwrap();
        match target {
            Target::Cell(cell_id) => {
                self.state.effects.push_back(Effect::MoveCardToCell {
                    card_id: card_id.clone(),
                    cell_id,
                });
                self.state.effects.extend(card.genesis());
            }
            Target::Card(_) => {
                let effects = card.on_cast(&self.state, target);
                self.state.effects.extend(effects);
            }
            _ => {}
        }
        self.state.effects.push_back(Effect::ChangePhase {
            new_phase: Phase::WaitingForPlay {
                player_id: player_id.clone(),
            },
        });

        let resolve_effects = card.after_resolve(&self.state);
        self.state.effects.extend(resolve_effects);

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
                    self.state.effects.push_back(Effect::ChangePhase {
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
