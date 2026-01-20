use crate::{
    card::{Ability, Card, CardData, CardType, MinionType, Region, SiteType, Zone},
    deck::Deck,
    effect::Effect,
    game::{Element, InputStatus, PlayerId, Resources},
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

#[derive(Debug, Default, Clone)]
pub struct CardMatcher {
    pub id: Option<uuid::Uuid>,
    pub controller_id: Option<PlayerId>,
    pub not_in_ids: Option<Vec<uuid::Uuid>>,
    pub abilities: Option<Vec<Ability>>,
    pub card_types: Option<Vec<CardType>>,
    pub minion_types: Option<Vec<MinionType>>,
    pub site_types: Option<Vec<SiteType>>,
    pub with_affinity: Option<Vec<Element>>,
    pub in_zones: Option<Vec<Zone>>,
    pub in_regions: Option<Vec<Region>>,
    pub tapped: Option<bool>,
    pub in_play: Option<bool>,
}

impl CardMatcher {
    pub fn from_id(id: uuid::Uuid) -> Self {
        Self {
            id: Some(id),
            ..Default::default()
        }
    }

    pub fn iter<'a>(&'a self, state: &'a State) -> impl Iterator<Item = &'a Box<dyn Card>> {
        state.cards.iter().filter(|c| self.matches(c.get_id(), state))
    }

    pub fn resolve_ids(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| self.matches(c.get_id(), state))
            .map(|c| c.get_id().clone())
            .collect()
    }

    pub fn units_in_zone(zones: Vec<Zone>) -> Self {
        Self {
            card_types: Some(vec![CardType::Minion, CardType::Avatar]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn in_zone(self, zone: &Zone) -> Self {
        Self {
            in_zones: Some(vec![zone.clone()]),
            ..self
        }
    }

    pub fn in_play(self, in_play: bool) -> Self {
        Self {
            in_play: Some(in_play),
            ..self
        }
    }

    pub fn tapped(self, tapped: bool) -> Self {
        Self {
            tapped: Some(tapped),
            ..self
        }
    }

    pub fn adjacent_to(self, zone: &Zone) -> Self {
        let zones = zone.get_adjacent();
        Self {
            in_zones: Some(zones),
            ..self
        }
    }

    pub fn near(self, zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            in_zones: Some(zones),
            ..self
        }
    }

    pub fn in_regions(self, region: Vec<Region>) -> Self {
        Self {
            in_regions: Some(region),
            ..self
        }
    }

    pub fn in_region(self, region: Region) -> Self {
        Self {
            in_regions: Some(vec![region]),
            ..self
        }
    }

    pub fn with_affinities(self, elements: Vec<Element>) -> Self {
        Self {
            with_affinity: Some(elements),
            ..self
        }
    }

    pub fn with_affinity(self, elements: Element) -> Self {
        Self {
            with_affinity: Some(vec![elements]),
            ..self
        }
    }

    pub fn sites_adjacent(zone: &Zone) -> Self {
        let zones = zone.get_adjacent();
        Self {
            card_types: Some(vec![CardType::Site]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn units_adjacent(zone: &Zone) -> Self {
        let zones = zone.get_adjacent();
        Self {
            card_types: Some(vec![CardType::Minion, CardType::Avatar]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn minions_adjacent(zone: &Zone) -> Self {
        let zones = zone.get_adjacent();
        Self {
            card_types: Some(vec![CardType::Minion]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn sites_near(zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            card_types: Some(vec![CardType::Site]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn units_near(zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            card_types: Some(vec![CardType::Minion, CardType::Avatar]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn minions_near(zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            card_types: Some(vec![CardType::Minion]),
            in_zones: Some(zones),
            ..Default::default()
        }
    }

    pub fn with_abilities(self, abilities: Vec<Ability>) -> Self {
        Self {
            abilities: Some(abilities),
            ..self
        }
    }

    pub fn controller_id(self, controller_id: &PlayerId) -> Self {
        Self {
            controller_id: Some(controller_id.clone()),
            ..self
        }
    }

    pub fn not_in_ids(self, not_in_ids: Vec<uuid::Uuid>) -> Self {
        Self {
            not_in_ids: Some(not_in_ids),
            ..self
        }
    }

    pub fn site_types(self, site_types: Vec<SiteType>) -> Self {
        Self {
            site_types: Some(site_types),
            ..self
        }
    }

    pub fn card_type(self, card_type: CardType) -> Self {
        Self {
            card_types: Some(vec![card_type]),
            ..self
        }
    }

    pub fn card_types(self, card_types: Vec<CardType>) -> Self {
        Self {
            card_types: Some(card_types),
            ..self
        }
    }

    pub fn matches(&self, card_id: &uuid::Uuid, state: &State) -> bool {
        let card = state.get_card(card_id);
        if let Some(id) = &self.id {
            if card_id != id {
                return false;
            }
        }

        if let Some(in_play) = &self.in_play {
            if &card.get_zone().is_in_play() != in_play {
                return false;
            }
        }

        if let Some(regions) = &self.in_regions {
            if !regions.contains(card.get_region(state)) {
                return false;
            }
        }

        if let Some(with_affinity) = &self.with_affinity {
            let mut has_affinity = false;
            for element in with_affinity {
                if card.get_elements(state).unwrap_or_default().contains(element) {
                    has_affinity = true;
                    break;
                }
            }

            if !has_affinity {
                return false;
            }
        }

        if let Some(tapped) = &self.tapped {
            if &card.is_tapped() != tapped {
                return false;
            }
        }

        if let Some(abilities) = &self.abilities {
            let card_abilities = card.get_abilities(state).unwrap_or_default();
            for ability in abilities {
                if !card_abilities.contains(ability) {
                    return false;
                }
            }
        }

        if let Some(not_in_ids) = &self.not_in_ids {
            if not_in_ids.contains(card_id) {
                return false;
            }
        }

        if let Some(controller_id) = &self.controller_id {
            if &card.get_controller_id(state) != controller_id {
                return false;
            }
        }

        if let Some(card_types) = &self.card_types {
            if !card_types.contains(&card.get_card_type()) {
                return false;
            }
        }

        if let Some(site_types) = &self.site_types {
            if let Some(base) = card.get_site_base() {
                let types = &base.types;
                let mut found_type = false;
                for site_type in site_types {
                    if types.contains(site_type) {
                        found_type = true;
                    }
                }

                if !found_type {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(minion_types) = &self.minion_types {
            if let Some(base) = card.get_unit_base() {
                let types = &base.types;
                let mut found_type = false;
                for minion_type in minion_types {
                    if types.contains(minion_type) {
                        found_type = true;
                    }
                }

                if !found_type {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(in_zones) = &self.in_zones {
            if !in_zones.contains(&card.get_zone()) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
pub enum ContinousEffect {
    ControllerOverride {
        controller_id: PlayerId,
        affected_cards: CardMatcher,
    },
    ModifyPower {
        power_diff: i16,
        affected_cards: CardMatcher,
    },
    FloodSites {
        affected_sites: CardMatcher,
    },
    ChangeSiteType {
        site_type: SiteType,
        affected_sites: CardMatcher,
    },
    GrantAbility {
        ability: Ability,
        affected_cards: CardMatcher,
    },
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
    pub continuous_effects: Vec<ContinousEffect>,
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
            continuous_effects: Vec::new(),
        }
    }

    pub fn get_body_of_water_size(&self, zone: &Zone) -> u16 {
        let mut visited: Vec<Zone> = Vec::new();
        let mut water_zones: Vec<Zone> = Vec::new();

        fn dfs(state: &State, zone: &Zone, visited: &mut Vec<Zone>, water_zones: &mut Vec<Zone>) {
            if visited.iter().any(|z| z == zone) {
                return;
            }
            visited.push(zone.clone());
            let sites = state.get_cards_in_zone(zone);
            let is_water = sites.iter().any(|card| {
                card.get_site_base()
                    .map(|base| base.provided_thresholds.water > 0)
                    .unwrap_or(false)
            });
            if is_water {
                if !water_zones.iter().any(|z| z == zone) {
                    water_zones.push(zone.clone());
                }
                for adj in zone.get_adjacent() {
                    dfs(state, &adj, visited, water_zones);
                }
            }
        }

        // Start DFS from adjacent zones
        for adj in zone.get_adjacent() {
            dfs(self, &adj, &mut visited, &mut water_zones);
        }

        water_zones.len() as u16
    }

    pub async fn compute_world_effects(&mut self) -> anyhow::Result<()> {
        self.continuous_effects.clear();

        for card in &self.cards {
            if !card.get_zone().is_in_play() {
                continue;
            }

            let card_world_effects = card.get_continuos_effects(self).await?;
            for effect in card_world_effects {
                self.continuous_effects.push(effect);
            }
        }

        Ok(())
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

    pub fn get_defenders_for_attack(&self, defender_id: &uuid::Uuid) -> Vec<uuid::Uuid> {
        let defender = self.get_card(defender_id);
        CardMatcher::units_near(defender.get_zone())
            .controller_id(&defender.get_controller_id(self))
            .resolve_ids(self)
    }

    pub fn get_interceptors_for_move(&self, path: &[Zone], controller_id: &PlayerId) -> Vec<(uuid::Uuid, Zone)> {
        self.cards
            .iter()
            .filter(|c| &c.get_controller_id(self) == controller_id)
            .filter(|c| c.is_unit())
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| path.contains(c.get_zone()))
            .map(|c| (c.get_id().clone(), c.get_zone().clone()))
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
                controller_id: c.get_controller_id(&self),
                tapped: c.is_tapped(),
                edition: c.get_edition().clone(),
                zone: c.get_zone().clone(),
                card_type: c.get_card_type().clone(),
                abilities: c.get_abilities(&self).unwrap_or_default(),
                region: c.get_region(&self).clone(),
                damage_taken: c.get_damage_taken().unwrap_or(0),
                bearer: c
                    .get_artifact()
                    .and_then(|c| c.get_bearer().unwrap_or_default().clone()),
                rarity: c.get_base().rarity.clone(),
                num_arts: c.get_num_arts(),
                power: c.get_power(&self).unwrap_or_default().unwrap_or_default(),
                has_attachments: c.has_attachments(self).unwrap_or_default(),
                image_path: c.get_image_path(),
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

    pub fn get_player(&self, player_id: &PlayerId) -> anyhow::Result<&Player> {
        Ok(self
            .players
            .iter()
            .find(|p| &p.id == player_id)
            .ok_or(anyhow::anyhow!("failed to get player deck"))?)
    }

    pub fn get_player_deck(&self, player_id: &PlayerId) -> anyhow::Result<&Deck> {
        Ok(self
            .decks
            .get(player_id)
            .ok_or(anyhow::anyhow!("failed to get player deck"))?)
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
            continuous_effects: self.continuous_effects.clone(),
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
