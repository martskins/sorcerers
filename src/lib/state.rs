use crate::{
    card::{Card, RenderableCard, Zone},
    deck::Deck,
    effect::Effect,
    game::{InputStatus, PlayerId, Resources},
    networking::message::{ClientMessage, ServerMessage},
};
use async_channel::{Receiver, Sender};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Phase {
    Main,
    PreEndTurn { player_id: PlayerId },
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
}

pub struct PlayerWithDeck {
    pub player: Player,
    pub deck: Deck,
    pub cards: Vec<Box<dyn Card>>,
}

#[derive(Debug)]
pub struct State {
    pub game_id: uuid::Uuid,
    pub players: Vec<Player>,
    pub turns: usize,
    pub cards: Vec<Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub resources: HashMap<PlayerId, Resources>,
    pub input_status: InputStatus,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub current_player: PlayerId,
    pub effects: VecDeque<Arc<Effect>>,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
}

impl State {
    pub fn new(
        game_id: uuid::Uuid,
        players_with_decks: Vec<PlayerWithDeck>,
        server_tx: Sender<ServerMessage>,
        client_rx: Receiver<ClientMessage>,
    ) -> Self {
        let mut cards: Vec<Box<dyn Card>> = Vec::new();
        let mut decks = HashMap::new();
        let resources = players_with_decks
            .iter()
            .map(|p| (p.player.id.clone(), Resources::new()))
            .collect();
        let players = players_with_decks.iter().map(|p| p.player.clone()).collect();
        let player_one = players_with_decks[0].player.id.clone();
        for player in players_with_decks {
            cards.extend(player.cards);
            decks.insert(player.player.id.clone(), player.deck);
        }

        State {
            game_id,
            players,
            cards,
            decks,
            turns: 0,
            resources,
            input_status: InputStatus::None,
            phase: Phase::Main,
            current_player: player_one,
            waiting_for_input: false,
            effects: VecDeque::new(),
            player_one,
            server_tx,
            client_rx,
        }
    }

    pub async fn apply_effects_without_log(&mut self) -> anyhow::Result<()> {
        while !self.effects.is_empty() {
            if self.waiting_for_input {
                return Ok(());
            }

            let effect = self.effects.pop_back();
            if let Some(effect) = effect {
                effect.apply(self).await?;
            }
        }

        Ok(())
    }

    fn renderables_from_cards(&self) -> Vec<RenderableCard> {
        self.cards
            .iter()
            .map(|c| RenderableCard {
                id: c.get_id().clone(),
                name: c.get_name().to_string(),
                owner_id: c.get_owner_id().clone(),
                tapped: c.is_tapped(),
                edition: c.get_edition().clone(),
                zone: c.get_zone().clone(),
                card_type: c.get_card_type().clone(),
                modifiers: c.get_modifiers(&self),
                plane: c.get_plane(&self).clone(),
                damage_taken: c.get_damage_taken(),
            })
            .collect()
    }

    pub fn into_sync(&self) -> ServerMessage {
        ServerMessage::Sync {
            cards: self.renderables_from_cards(),
            resources: self.resources.clone(),
            current_player: self.current_player.clone(),
        }
    }

    pub fn get_receiver(&self) -> Receiver<ClientMessage> {
        self.client_rx.clone()
    }

    pub fn get_sender(&self) -> Sender<ServerMessage> {
        self.server_tx.clone()
    }

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> Option<&mut Box<dyn Card>> {
        self.cards.iter_mut().find(|c| c.get_id() == card_id)
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> Option<&Box<dyn Card>> {
        self.cards.iter().find(|c| c.get_id() == card_id)
    }

    pub fn get_minions_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards
            .iter()
            .filter(|c| c.get_zone() == zone)
            .filter(|c| c.is_minion())
            .collect()
    }

    pub fn get_units_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards
            .iter()
            .filter(|c| c.get_zone() == zone)
            .filter(|c| c.is_unit())
            .collect()
    }

    pub fn get_cards_in_zone(&self, zone: &Zone) -> Vec<&Box<dyn Card>> {
        self.cards.iter().filter(|c| c.get_zone() == zone).collect()
    }

    pub fn get_player_resources(&self, player_id: &PlayerId) -> &Resources {
        self.resources.get(player_id).unwrap()
    }

    pub fn snapshot(&self) -> State {
        State {
            game_id: self.game_id.clone(),
            players: self.players.clone(),
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: 0,
            resources: self.resources.clone(),
            input_status: self.input_status.clone(),
            phase: self.phase.clone(),
            current_player: self.current_player,
            waiting_for_input: self.waiting_for_input,
            effects: self.effects.clone(),
            player_one: self.player_one,
            server_tx: self.server_tx.clone(),
            client_rx: self.client_rx.clone(),
        }
    }
}
