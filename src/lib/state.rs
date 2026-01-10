use crate::{
    card::{Card, CardData, Zone},
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
    // PreEndTurn { player_id: PlayerId },
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
    pub effect_log: Vec<Arc<Effect>>,
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
            effect_log: Vec::new(),
            player_one,
            server_tx,
            client_rx,
        }
    }

    pub fn get_player_avatar_id(&self, player_id: &PlayerId) -> anyhow::Result<uuid::Uuid> {
        self.decks
            .get(player_id)
            .and_then(|d| Some(d.avatar.clone()))
            .ok_or(anyhow::anyhow!("failed to get player avatar id"))
    }

    pub fn get_opponent_id(&self, player_id: &PlayerId) -> anyhow::Result<PlayerId> {
        for player in &self.players {
            if &player.id != player_id {
                return Ok(player.id.clone());
            }
        }

        Err(anyhow::anyhow!("failed to get opponent id"))
    }

    pub fn get_interceptors_for_move(&self, path: &[Zone], controller_id: &PlayerId) -> Vec<(uuid::Uuid, Zone)> {
        self.cards
            .iter()
            .filter(|c| c.get_controller_id() == controller_id)
            .filter(|c| c.is_unit())
            .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
            .flat_map(|c| {
                let valid_moves = c.get_valid_move_zones(self).unwrap_or_default();
                let mut intercepts = vec![];
                for zone in path {
                    if valid_moves.contains(&zone) {
                        intercepts.push((c.get_id().clone(), zone.clone()));
                    }
                }

                intercepts
            })
            .collect()
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

    pub fn data_from_cards(&self) -> Vec<CardData> {
        self.cards
            .iter()
            // TODO: filter only cards in play
            // .filter_map(|c| match c.get_zone() {
            //     Zone::Hand | Zone::Realm(_) | Zone::Intersection(_) => Some(c),
            //     _ => return None,
            // })
            .map(|c| CardData {
                id: c.get_id().clone(),
                name: c.get_name().to_string(),
                owner_id: c.get_owner_id().clone(),
                tapped: c.is_tapped(),
                edition: c.get_edition().clone(),
                zone: c.get_zone().clone(),
                card_type: c.get_card_type().clone(),
                abilities: c.get_modifiers(&self).unwrap_or_default(),
                plane: c.get_plane(&self).clone(),
                damage_taken: c.get_damage_taken().unwrap_or(0),
                bearer: c
                    .get_artifact()
                    .and_then(|c| c.get_bearer().unwrap_or_default().clone()),
                rarity: c.get_base().rarity.clone(),
                num_arts: c.get_num_arts(),
                power: c.get_power(&self).unwrap_or_default().unwrap_or_default(),
            })
            .collect()
    }

    pub fn into_sync(&self) -> anyhow::Result<ServerMessage> {
        let mut health = HashMap::new();
        for player in &self.players {
            let avatar_id = self.get_player_avatar_id(&player.id)?;
            let avatar_card = self.get_card(&avatar_id);
            health.insert(
                player.id.clone(),
                avatar_card
                    .get_unit_base()
                    .ok_or(anyhow::anyhow!("no unit base in avatar"))?
                    .toughness
                    - avatar_card.get_damage_taken().unwrap_or(0),
            );
        }

        Ok(ServerMessage::Sync {
            cards: self.data_from_cards(),
            resources: self.resources.clone(),
            current_player: self.current_player.clone(),
            health: health,
        })
    }

    pub fn get_receiver(&self) -> Receiver<ClientMessage> {
        self.client_rx.clone()
    }

    pub fn get_sender(&self) -> Sender<ServerMessage> {
        self.server_tx.clone()
    }

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> &mut Box<dyn Card> {
        self.cards
            .iter_mut()
            .find(|c| c.get_id() == card_id)
            .expect("failed to get card")
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> &Box<dyn Card> {
        self.cards
            .iter()
            .find(|c| c.get_id() == card_id)
            .expect("failed to get card")
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

    pub fn get_player_resources_mut(&mut self, player_id: &PlayerId) -> anyhow::Result<&mut Resources> {
        Ok(self
            .resources
            .get_mut(player_id)
            .ok_or(anyhow::anyhow!("failed to get player resources"))?)
    }

    pub fn get_player_resources(&self, player_id: &PlayerId) -> anyhow::Result<&Resources> {
        Ok(self
            .resources
            .get(player_id)
            .ok_or(anyhow::anyhow!("failed to get player resources"))?)
    }

    pub fn snapshot(&self) -> State {
        State {
            game_id: self.game_id.clone(),
            players: self.players.clone(),
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: self.turns.clone(),
            resources: self.resources.clone(),
            input_status: self.input_status.clone(),
            phase: self.phase.clone(),
            current_player: self.current_player,
            waiting_for_input: self.waiting_for_input,
            effects: self.effects.clone(),
            player_one: self.player_one,
            server_tx: self.server_tx.clone(),
            client_rx: self.client_rx.clone(),
            effect_log: self.effect_log.clone(),
        }
    }

    #[cfg(test)]
    pub fn new_mock_state(zones_with_sites: impl AsRef<[Zone]>) -> State {
        use crate::card::from_name_and_zone;

        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let cards: Vec<Box<dyn Card>> = zones_with_sites
            .as_ref()
            .into_iter()
            .map(|z| from_name_and_zone("Arid Desert", &player_one_id, z.clone()))
            .collect();

        let player1 = PlayerWithDeck {
            player: Player {
                id: player_one_id.clone(),
                name: "Player 1".to_string(),
            },
            deck: Deck::new(&player_one_id, vec![], vec![], uuid::Uuid::nil()),
            cards,
        };
        let player2 = PlayerWithDeck {
            player: Player {
                id: player_two_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(&player_two_id, vec![], vec![], uuid::Uuid::nil()),
            cards: vec![],
        };

        let players = vec![player1, player2];
        let (server_tx, _) = async_channel::unbounded();
        let (_, client_rx) = async_channel::unbounded();
        State::new(uuid::Uuid::new_v4(), players, server_tx, client_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{HeadlessHaunt, KiteArcher, NimbusJinn, RimlandNomads};

    #[test]
    fn test_inteceptors() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut rimland_nomads = RimlandNomads::new(player_id.clone());
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id.clone();
        let mut kite_archer = KiteArcher::new(opponent_id.clone());
        kite_archer.set_zone(Zone::Realm(12));
        state.cards.push(Box::new(kite_archer.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 1);
        assert_eq!(&interceptors[0].0, kite_archer.get_id());
    }

    #[test]
    fn test_no_inteceptors() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut rimland_nomads = RimlandNomads::new(player_id.clone());
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id.clone();
        let mut kite_archer = KiteArcher::new(opponent_id.clone());
        kite_archer.set_zone(Zone::Realm(11));
        state.cards.push(Box::new(kite_archer.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 0);
    }

    #[test]
    fn test_voidwalking_interceptor() {
        let mut state = State::new_mock_state(vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)]);
        let player_id = state.players[0].id.clone();
        let mut rimland_nomads = RimlandNomads::new(player_id.clone());
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id.clone();
        let mut headless_haunt = HeadlessHaunt::new(opponent_id.clone());
        headless_haunt.set_zone(Zone::Realm(12));
        state.cards.push(Box::new(headless_haunt.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 1);
    }

    #[test]
    fn test_airborne_interceptor() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut rimland_nomads = RimlandNomads::new(player_id.clone());
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id.clone();
        let mut headless_haunt = NimbusJinn::new(opponent_id.clone());
        headless_haunt.set_zone(Zone::Realm(12));
        state.cards.push(Box::new(headless_haunt.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 3);
    }
}
