use crate::{
    card::{
        avatar::Avatar,
        spell::{Spell, SpellType},
        Card, CardBase, CardType, CardZone, Target,
    },
    deck::{precon_beta, Deck},
    effect::{Action, Effect, GameAction},
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
    SelectingSquare {
        player_id: uuid::Uuid,
        square: Vec<u8>,
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
    SelectingCardOutsideRealm {
        player_id: uuid::Uuid,
        owner: Option<uuid::Uuid>,
        spellbook: Option<Vec<uuid::Uuid>>,
        cemetery: Option<Vec<uuid::Uuid>>,
        hand: Option<Vec<uuid::Uuid>>,
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
    pub resources: HashMap<uuid::Uuid, Resources>,
    #[serde(skip)]
    pub decks: HashMap<uuid::Uuid, Deck>,
    #[serde(skip)]
    pub actions: HashMap<uuid::Uuid, Vec<Action>>,
    #[serde(skip)]
    pub effects: VecDeque<Effect>,
}

impl State {
    pub fn new(players: Vec<uuid::Uuid>) -> Self {
        let mut decks = HashMap::new();
        if players.len() >= 2 {
            let player1 = players[0];
            let player2 = players[1];
            let deck_one = precon_beta::fire(player1);
            println!("{:?}", deck_one.avatar.get_edition());
            let deck_two = precon_beta::fire(player2);
            decks.insert(player1, deck_one);
            decks.insert(player2, deck_two);
        }

        State {
            phase: Phase::None,
            turns_taken: 0,
            current_player: uuid::Uuid::nil(),
            players,
            cards: vec![],
            effects: VecDeque::new(),
            resources: HashMap::new(),
            actions: HashMap::new(),
            decks,
        }
    }

    pub fn is_players_turn(&self, player_id: &uuid::Uuid) -> bool {
        &self.current_player == player_id
    }

    fn is_square_empty(&self, square: &u8) -> bool {
        !self
            .cards
            .iter()
            .filter(|c| c.get_type() != CardType::Avatar)
            .any(|card| match card.get_zone() {
                CardZone::Realm(id) => id == square,
                _ => false,
            })
    }

    pub fn valid_play_cells(&self, card: &Card) -> Vec<u8> {
        let adjacent_cells = self.get_cells_adjacent_to_sites(&card.get_owner_id());
        match card.get_type() {
            CardType::Site => {
                let mut cells = vec![];
                for cell in &adjacent_cells {
                    if self.is_square_empty(cell) {
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

    pub fn get_cards_in_cell(&self, square: &u8) -> Vec<&Card> {
        self.cards
            .iter()
            .filter(|card| match card.get_zone() {
                CardZone::Realm(id) => id == square,
                _ => false,
            })
            .collect()
    }

    pub fn get_cells_adjacent_to_sites(&self, owner_id: &uuid::Uuid) -> Vec<u8> {
        if !self.has_played_site(owner_id) {
            let starting_cell = if self.is_player_one(owner_id) { 3 } else { 18 };
            return vec![starting_cell];
        }

        let mut adjacent_cells = Vec::new();
        let site_squares: Vec<u8> = self
            .cards
            .iter()
            .filter(|card| card.get_owner_id() == owner_id && card.is_site())
            .filter_map(|card| match card.get_zone() {
                CardZone::Realm(square) => Some(*square),
                _ => None,
            })
            .collect();

        for square in site_squares {
            let neighbors = Self::get_adjacent_squares(square);
            for neighbor in neighbors {
                if !adjacent_cells.contains(&neighbor) {
                    adjacent_cells.push(neighbor);
                }
            }
        }

        adjacent_cells
    }

    /// Returns the ids of the cells that are directly above, below, left or right of the given
    /// cell id.
    pub fn get_nearby_squares(cell_id: u8) -> Vec<u8> {
        let mut nearby = Vec::new();
        let rows = 4;
        let cols = 5;
        let row = cell_id / cols;
        let col = cell_id % cols;

        if row > 0 {
            nearby.push((row - 1) * cols + col);
        }
        if row < rows - 1 {
            nearby.push((row + 1) * cols + col);
        }
        if col > 0 {
            nearby.push(row * cols + (col - 1));
        }
        if col < cols - 1 {
            nearby.push(row * cols + (col + 1));
        }

        let diagonal_offsets = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        for (dr, dc) in diagonal_offsets {
            let new_row = row as i8 + dr;
            let new_col = col as i8 + dc;
            if new_row >= 0 && new_row < rows as i8 && new_col >= 0 && new_col < cols as i8 {
                nearby.push((new_row as u8) * cols + (new_col as u8));
            }
        }
        nearby
    }

    pub fn get_adjacent_squares(cell_id: u8) -> Vec<u8> {
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

    pub fn get_playable_site_ids(&self, player_id: &uuid::Uuid) -> Vec<uuid::Uuid> {
        self.cards
            .iter()
            .filter(|card| card.get_owner_id() == player_id && card.is_site())
            .filter(|card| matches!(card.get_zone(), CardZone::Hand))
            .map(|card| card.get_id())
            .cloned()
            .collect()
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
        self.cards.push(card);

        match self.phase {
            Phase::WaitingForCardDraw { ref mut count, .. } => {
                *count -= 1;

                if *count == 0 {
                    self.effects.push_back(Effect::ChangePhase {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Square {
    pub id: u8,
    pub occupied_by: Vec<Card>,
}

impl Square {
    pub fn are_adjacent(a: u8, b: u8) -> bool {
        let adjacent_cells = State::get_adjacent_squares(a);
        adjacent_cells.contains(&b)
    }

    pub fn are_nearby(a: u8, b: u8) -> bool {
        let adjacent_cells = State::get_nearby_squares(a);
        adjacent_cells.contains(&b)
    }
}

pub struct Game {
    pub id: uuid::Uuid,
    pub players: Vec<uuid::Uuid>,
    pub state: State,
    pub addrs: HashMap<uuid::Uuid, Socket>,
    pub socket: Arc<UdpSocket>,
}

impl Game {
    pub fn new(player1: uuid::Uuid, player2: uuid::Uuid, socket: Arc<UdpSocket>, addr1: Socket, addr2: Socket) -> Self {
        let state = State::new(vec![player1, player2]);
        Game {
            id: uuid::Uuid::new_v4(),
            players: vec![player1, player2],
            state,
            socket,
            addrs: HashMap::from([(player1, addr1), (player2, addr2)]),
        }
    }

    pub async fn process_message(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::PrepareCardForPlay { card_id, player_id, .. } => {
                self.prepare_card_for_play(&player_id, &card_id).await?
            }
            Message::PlayCard {
                card_id,
                player_id,
                targets,
                ..
            } => self.play_card(&player_id, &card_id, targets).await?,
            Message::SelectCard { card_id, player_id, .. } => self.select_card(&player_id, &card_id).await?,
            Message::EndTurn { player_id, .. } => self.end_turn(&player_id).await?,
            Message::DrawCard {
                card_type, player_id, ..
            } => self.state.draw_card_for_player(&player_id, card_type).await?,
            Message::TriggerAction {
                player_id, action_idx, ..
            } => self.trigger_action(&player_id, action_idx).await?,
            Message::CancelSelectAction { player_id, .. } => {
                self.state.phase = Phase::WaitingForPlay { player_id };
            }
            Message::AttackTarget {
                attacker_id, target_id, ..
            } => self.attack_target(&attacker_id, &target_id).await?,
            Message::MoveCard { card_id, square, .. } => self.move_card(&card_id, square).await?,
            Message::SummonMinion { card_id, square, .. } => {
                self.summon_minion(&card_id, square, &self.state.clone()).await?
            }
            _ => {}
        }

        self.update().await?;
        Ok(())
    }

    async fn summon_minion(&mut self, card_id: &uuid::Uuid, square: u8, state: &State) -> anyhow::Result<()> {
        let mut effects = vec![
            Effect::MoveCardToSquare {
                card_id: card_id.clone(),
                square,
            },
            Effect::ChangePhase {
                new_phase: Phase::WaitingForPlay {
                    player_id: self.state.current_player.clone(),
                },
            },
        ];
        let genesis_effects = self
            .state
            .cards
            .iter()
            .find(|c| c.get_id() == card_id)
            .unwrap()
            .genesis(state);
        effects.extend(genesis_effects);
        // match cell_id {
        //     Some(id) => {
        //     }
        //     None => {
        //         self.state.effects.push_back(Effect::ChangePhase {
        //             new_phase: Phase::SelectingCell {
        //                 player_id: self.state.current_player.clone(),
        //                 cell_ids: self.state.valid_play_cells(&self.get_card_by_id(&card_id).unwrap()),
        //                 after_select: Some(Action::GameAction(GameAction::SummonMinionToSelectedCell {
        //                     card_id: card_id.clone(),
        //                 })),
        //             },
        //         });
        //     }
        // };
        //
        Ok(())
    }

    async fn move_card(&mut self, card_id: &uuid::Uuid, square: u8) -> anyhow::Result<()> {
        self.state.effects.push_back(Effect::MoveCardToSquare {
            card_id: card_id.clone(),
            square,
        });
        self.state.effects.push_back(Effect::ChangePhase {
            new_phase: Phase::WaitingForPlay {
                player_id: self.state.current_player.clone(),
            },
        });
        Ok(())
    }

    fn get_card_by_id_mut(&mut self, card_id: &uuid::Uuid) -> Option<&mut Card> {
        self.state.cards.iter_mut().find(|card| card.get_id() == card_id)
    }

    fn get_card_by_id(&self, card_id: &uuid::Uuid) -> Option<&Card> {
        self.state.cards.iter().find(|card| card.get_id() == card_id)
    }

    fn check_damage(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self
            .state
            .cards
            .iter_mut()
            .filter(|c| c.is_minion())
            .filter(|c| c.is_in_realm())
            .filter(|c| match c {
                Card::Spell(spell) => spell.is_dead(),
                Card::Site(_) => false,
                Card::Avatar(_) => false,
            })
            .map(|c| c.get_id().clone())
            .collect();

        for card_id in card_ids {
            self.state.effects.push_back(Effect::KillUnit { card_id });
        }
    }

    async fn attack_target(&mut self, attacker_id: &uuid::Uuid, target_id: &uuid::Uuid) -> anyhow::Result<()> {
        let target_square = self.get_card_by_id(&target_id).unwrap().get_square().unwrap();
        let mut effects = vec![
            Effect::TapCard {
                card_id: attacker_id.clone(),
            },
            Effect::MoveCardToSquare {
                card_id: attacker_id.clone(),
                square: target_square,
            },
        ];

        match self.get_card_by_id(&attacker_id).unwrap() {
            Card::Spell(spell) => match spell.get_spell_type() {
                SpellType::Minion => {
                    let power = spell.get_power();
                    if let Some(power) = power {
                        let target = self.get_card_by_id_mut(&target_id).unwrap();
                        effects.extend(target.take_damage(attacker_id, power));
                    }
                }
                _ => {}
            },
            _ => {}
        }

        match self.get_card_by_id(&target_id).unwrap() {
            Card::Spell(spell) => match spell.get_spell_type() {
                SpellType::Minion => {
                    let power = spell.get_power();
                    if let Some(power) = power {
                        let attacker = self.get_card_by_id_mut(&attacker_id).unwrap();
                        effects.extend(attacker.take_damage(target_id, power));
                    }
                }
                _ => {}
            },
            _ => {}
        }

        self.state.effects.extend(effects);
        self.state.effects.push_back(Effect::ChangePhase {
            new_phase: Phase::WaitingForPlay {
                player_id: self.state.current_player.clone(),
            },
        });
        Ok(())
    }

    async fn trigger_action(&mut self, player_id: &uuid::Uuid, action_idx: usize) -> anyhow::Result<()> {
        let actions = self.state.actions.get(player_id).unwrap();
        if action_idx >= actions.len() {
            return Ok(());
        }

        let after_select = actions[action_idx].after_select_effects();
        self.state.effects.extend(after_select.clone());
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        while let Some(effect) = self.state.effects.pop_front() {
            effect.apply(&mut self.state).await;
        }
        self.check_damage();
        self.send_sync().await?;
        Ok(())
    }

    pub fn place_avatars(&mut self) -> anyhow::Result<()> {
        for player_id in &self.players {
            let deck = self.state.decks.get_mut(&player_id).unwrap();
            let mut avatar_card = Card::Avatar(deck.avatar.clone());
            let square = if self.state.is_player_one(player_id) { 3 } else { 18 };
            avatar_card.set_zone(CardZone::Realm(square));
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
        let deck = self.state.decks.get_mut(player_id).unwrap();
        deck.shuffle();

        self.state.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.state.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.state.draw_card_for_player(&player_id, CardType::Spell).await?;
        self.state.draw_card_for_player(&player_id, CardType::Site).await?;
        self.state.draw_card_for_player(&player_id, CardType::Site).await?;
        self.state.draw_card_for_player(&player_id, CardType::Site).await?;
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

    pub async fn select_card(&mut self, player_id: &uuid::Uuid, card_id: &uuid::Uuid) -> anyhow::Result<()> {
        let state_clone = self.state.clone();
        let card = self
            .state
            .cards
            .iter_mut()
            .find(|card| card.get_id() == card_id && card.get_owner_id() == player_id);
        if card.is_none() {
            return Ok(());
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

    pub async fn play_card(
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
        let effects = card.on_cast(&self.state, target);
        self.state.effects.extend(effects);
        // match target {
        //     Target::Cell(cell_id) => {
        //         self.state.effects.push_back(Effect::MoveCardToCell {
        //             card_id: card_id.clone(),
        //             cell_id,
        //         });
        //         self.state.effects.extend(card.genesis());
        //     }
        //     Target::Card(_) => {
        //         let effects = card.on_cast(&self.state, target);
        //         self.state.effects.extend(effects);
        //     }
        //     _ => {}
        // }
        self.state.effects.push_back(Effect::ChangePhase {
            new_phase: Phase::WaitingForPlay {
                player_id: player_id.clone(),
            },
        });

        let resolve_effects = card.after_resolve(&self.state);
        self.state.effects.extend(resolve_effects);

        Ok(())
    }
}
