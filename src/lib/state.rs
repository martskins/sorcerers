use crate::{
    card::{Ability, Card, CardData, CardType, Costs, DodgeRoll, SiteType},
    deck::Deck,
    effect::{Effect, EffectCallback, EffectEngine, EffectState},
    game::{
        ActivatedAbility, InputStatus, PlayerId, Resources, Thresholds, ThresholdsDiff, pick_zone,
        yes_or_no,
    },
    networking::message::{ClientMessage, ServerMessage},
    query::{CardQuery, EffectQuery, ZoneQuery},
    zone::Zone,
};
use async_channel::{Receiver, Sender};
use std::collections::{HashMap, HashSet, VecDeque};

pub use crate::effect::{DeferredEffect, LoggedEffect, TemporaryEffect};

#[derive(Debug, PartialEq, Clone)]
pub enum Phase {
    Mulligan,
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

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum ContinuousEffect {
    ControllerOverride {
        controller_id: PlayerId,
        affected_cards: CardQuery,
    },
    ModifyPower {
        power_diff: i16,
        affected_cards: CardQuery,
    },
    FloodSites {
        affected_sites: CardQuery,
    },
    DroughtSites {
        affected_sites: CardQuery,
    },
    MakeZoneUnvisitable {
        affected_zone: Zone,
        affected_cards: CardQuery,
    },
    DoubleDamageTaken {
        affected_cards: CardQuery,
        except_strikes: bool,
    },
    ReduceDamageTaken {
        amount: u16,
        affected_cards: CardQuery,
    },
    PreventDamageFromMagic {
        affected_cards: CardQuery,
    },
    RestrictMoveToZones {
        affected_cards: CardQuery,
        allowed_zones: Vec<Zone>,
    },
    BlockMovementThrough {
        border: Zone,
        affected_cards: CardQuery,
    },
    ConnectTopBottomEdges {
        affected_cards: CardQuery,
    },
    ConnectLeftRightEdges {
        affected_cards: CardQuery,
    },
    ConnectZones {
        connected_zones: Vec<Zone>,
        affected_cards: CardQuery,
    },
    ChangeSiteType {
        site_type: SiteType,
        affected_sites: CardQuery,
    },
    GrantAbility {
        ability: Ability,
        affected_cards: CardQuery,
    },
    GrantActivatedAbility {
        ability: Box<dyn ActivatedAbility>,
        affected_cards: CardQuery,
    },
    ModifyProvidedAffinities {
        new_affinities: Thresholds,
        affected_sites: CardQuery,
    },
    ModifyProvidedMana {
        mana_diff: i8,
        affected_cards: CardQuery,
    },
    OverrideValidPlayZone {
        affected_zones: Vec<Zone>,
        affected_cards: CardQuery,
    },
    ModifyManaCost {
        mana_diff: i8,
        affected_cards: CardQuery,
        zones: Option<Vec<Zone>>,
    },
    TriggeredEffect {
        trigger_on_effect: EffectQuery,
        on_effect: EffectCallback,
    },
}

impl std::fmt::Debug for ContinuousEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TriggeredEffect {
                trigger_on_effect, ..
            } => f
                .debug_struct("AddTriggeredEffect")
                .field("trigger_on_effect", trigger_on_effect)
                .finish(),
            _ => std::fmt::Debug::fmt(self, f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Turn {
    pub(crate) player_id: PlayerId,
    pub(crate) controller_override: Option<PlayerId>,
}

impl Turn {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            player_id,
            controller_override: None,
        }
    }

    pub fn controlled_by(player_id: PlayerId, controller_id: PlayerId) -> Self {
        Self {
            player_id,
            controller_override: (player_id != controller_id).then_some(controller_id),
        }
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn controller_override(&self) -> Option<PlayerId> {
        self.controller_override
    }

    pub fn controller_id(&self) -> PlayerId {
        self.controller_override.unwrap_or(self.player_id)
    }
}

#[derive(Debug, Clone)]
pub struct TurnIterator {
    current: Turn,
    normal: Vec<Turn>,
    index: usize,
    overrides: VecDeque<Turn>,
}

impl TurnIterator {
    pub fn new(normal: Vec<PlayerId>) -> Self {
        assert!(!normal.is_empty());

        let normal: Vec<Turn> = normal.into_iter().map(Turn::new).collect();
        Self {
            current: normal[0].clone(),
            normal,
            index: 1,
            overrides: VecDeque::new(),
        }
    }

    pub fn current(&self) -> &Turn {
        &self.current
    }

    pub fn next_turn(&self) -> &Turn {
        self.overrides
            .front()
            .unwrap_or_else(|| &self.normal[self.index])
    }

    pub fn override_next(&mut self, value: Turn) {
        self.overrides.push_front(value);
    }

    pub fn override_upcoming<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Turn>,
    {
        self.overrides.extend(values);
    }
}

impl Iterator for TurnIterator {
    type Item = Turn;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.overrides.pop_front() {
            self.current = value;
            return Some(self.current.clone());
        }

        let value = self.normal[self.index].clone();
        self.index = (self.index + 1) % self.normal.len();
        self.current = value;
        Some(self.current.clone())
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub game_id: uuid::Uuid,
    pub players: Vec<Player>,
    pub turns: usize,
    pub cards: HashMap<uuid::Uuid, Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub input_status: InputStatus,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub curr_turn: TurnIterator,
    pub effects: EffectState,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
    pub continuous_effects: Vec<ContinuousEffect>,
    pub player_mana: HashMap<PlayerId, u8>,
    pub loosers: HashSet<PlayerId>,
    pub players_skipping_turns: HashSet<PlayerId>,
    pub players_with_accepted_hands: HashSet<PlayerId>,
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
        let players = players_with_decks
            .iter()
            .map(|p| p.player.clone())
            .collect();
        let player_mana = players_with_decks
            .iter()
            .map(|p| (p.player.id, 0))
            .collect();
        let player_one = players_with_decks[0].player.id;
        let mut player_ids = vec![];
        for player in players_with_decks {
            cards.extend(player.cards);
            decks.insert(player.player.id, player.deck);
            player_ids.push(player.player.id);
        }

        State {
            game_id,
            players,
            cards: cards.into_iter().map(|c| (*c.get_id(), c)).collect(),
            decks,
            turns: 0,
            input_status: InputStatus::None,
            phase: Phase::Mulligan,
            curr_turn: TurnIterator::new(player_ids),
            waiting_for_input: false,
            effects: EffectState::default(),
            player_one,
            server_tx,
            client_rx,
            continuous_effects: Vec::new(),
            player_mana,
            loosers: HashSet::new(),
            players_skipping_turns: HashSet::new(),
            players_with_accepted_hands: HashSet::new(),
        }
    }

    /// validate_client_message checks that the message contains valid references to game entities
    /// (e.g. card ids). It should be called before processing a message from the client, and can be
    /// used to catch client bugs or malicious messages.
    pub(super) fn validate_client_message(&self, msg: &ClientMessage) -> anyhow::Result<()> {
        // Validate that the message is for the correct game.
        if self.game_id != msg.game_id() {
            return Err(anyhow::anyhow!(
                "message game id {} does not match state game id {}",
                msg.game_id(),
                self.game_id
            ));
        }

        // Validate that the player is in the game.
        if self
            .players
            .iter()
            .find(|p| &p.id == msg.player_id())
            .is_none()
        {
            return Err(anyhow::anyhow!(
                "message player id {} does not match any player in state",
                msg.player_id()
            ));
        }

        // Validate that all cards mentioned in the message exist in the game.
        match msg {
            ClientMessage::ClickCard { card_id, .. }
            | ClientMessage::RequestPlayableZones { card_id, .. }
            | ClientMessage::PlayCardAtZone { card_id, .. }
            | ClientMessage::PickCard { card_id, .. } => {
                self.cards
                    .get(card_id)
                    .ok_or(anyhow::anyhow!("invalid card id"))?;
                Ok(())
            }
            ClientMessage::PickCards { card_ids, .. } => {
                for card_id in card_ids {
                    self.cards
                        .get(card_id)
                        .ok_or(anyhow::anyhow!("invalid card id"))?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn find_caster(&self, spell_id: &uuid::Uuid) -> Option<uuid::Uuid> {
        self.effect_log().iter().find_map(|e| match e.effect {
            Effect::PlayMagic {
                card_id, caster_id, ..
            } if card_id == *spell_id => Some(caster_id),
            _ => None,
        })
    }

    pub fn current_turn(&self) -> &Turn {
        self.curr_turn.current()
    }

    pub fn current_player(&self) -> PlayerId {
        self.current_turn().player_id()
    }

    pub fn current_turn_controller(&self) -> PlayerId {
        self.current_turn().controller_id()
    }

    pub fn decision_player(&self, player_id: impl AsRef<PlayerId>) -> PlayerId {
        if player_id.as_ref() == &self.current_player() {
            self.current_turn_controller()
        } else {
            *player_id.as_ref()
        }
    }

    pub fn advance_turn(&mut self) -> Turn {
        self.curr_turn
            .next()
            .expect("turn iterator should produce turns forever")
    }

    pub fn advance_to_turn(&mut self, player_id: &PlayerId) -> anyhow::Result<Turn> {
        if &self.current_player() != player_id {
            let turn = self.advance_turn();
            if &turn.player_id() != player_id {
                return Err(anyhow::anyhow!(
                    "expected next turn for player {}, got {}",
                    player_id,
                    turn.player_id()
                ));
            }
        }

        Ok(self.current_turn().clone())
    }

    pub fn next_turn(&self) -> &Turn {
        self.curr_turn.next_turn()
    }

    pub fn override_next_turn(&mut self, turn: Turn) {
        self.curr_turn.override_next(turn);
    }

    pub async fn replace_effect(&self, effect: &Effect) -> anyhow::Result<Option<Vec<Effect>>> {
        match effect {
            Effect::Attack {
                defender_id,
                attacker_id,
            } => {
                let defender = self.get_card(defender_id);
                if !defender.is_unit() {
                    return Ok(None);
                }

                let defender_controller = defender.get_controller_id(self);
                let dodge_rolls_in_hand = CardQuery::new()
                    .cards_named(DodgeRoll::NAME)
                    .controlled_by(&defender_controller)
                    .in_zone(&Zone::Hand)
                    .all(self);
                if dodge_rolls_in_hand.is_empty() {
                    return Ok(None);
                }

                let prompt = format!(
                    "Use Dodge Roll to evade the attack on {}?",
                    defender.get_name()
                );
                let use_dodge_roll = yes_or_no(defender_controller, self, prompt).await?;
                if !use_dodge_roll {
                    return Ok(None);
                }

                let avatar_id = self.get_player_avatar_id(&defender_controller)?;
                let avatar = self.get_card(&avatar_id);
                let adjacent_zones = defender.get_zone().get_adjacent();
                let prompt = "Dodge Roll: Pick an adjacent site to move to";
                let picked_site =
                    pick_zone(defender_controller, &adjacent_zones, self, true, prompt).await?;

                let attacker = self.get_card(attacker_id);
                let attacker_controller = attacker.get_controller_id(self);
                Ok(Some(vec![
                    Effect::SetCardZone {
                        card_id: *defender_id,
                        zone: picked_site,
                    },
                    Effect::MoveCard {
                        player_id: attacker_controller,
                        card_id: *attacker_id,
                        from: attacker.get_zone().clone(),
                        to: ZoneQuery::from_zone(defender.get_zone().clone()),
                        tap: true,
                        region: attacker.get_region(self).clone(),
                        through_path: None,
                    },
                    Effect::PlayMagic {
                        player_id: defender_controller,
                        card_id: dodge_rolls_in_hand[0],
                        caster_id: avatar_id,
                        from: avatar.get_zone().clone(),
                    },
                ]))
            }
            _ => Ok(None),
        }
    }

    pub fn get_player_mana_mut(&mut self, player_id: &PlayerId) -> &mut u8 {
        self.player_mana.entry(*player_id).or_insert(0)
    }

    /// Returns the effective play costs for a card after applying all matching
    /// `ModifyManaCost` continuous effects.
    ///
    /// `target_zone` is the zone the card is about to be placed in (e.g. the
    /// realm zone chosen by the player). Effects whose `zones` filter does not
    /// include `target_zone` are skipped; effects with `zones: None` are always
    /// applied.
    pub fn get_effective_costs(
        &self,
        card_id: &uuid::Uuid,
        target_zone: Option<&Zone>,
        player_id: &PlayerId,
    ) -> anyhow::Result<Costs> {
        let card = self.get_card(card_id);
        let base_costs = card.get_costs(self)?.clone();

        let total_mana_diff: i8 = self
            .continuous_effects
            .iter()
            .filter_map(|ce| match ce {
                ContinuousEffect::ModifyManaCost {
                    mana_diff,
                    affected_cards,
                    zones,
                } => {
                    if !affected_cards.matches(card.get_id(), self) {
                        return None;
                    }
                    let zone_ok = match zones {
                        None => true,
                        Some(effect_zones) => match target_zone {
                            None => false,
                            Some(z) => effect_zones.contains(z),
                        },
                    };
                    if zone_ok { Some(*mana_diff) } else { None }
                }
                _ => None,
            })
            .sum();

        let ignore_thresholds = self
            .temporary_effects()
            .iter()
            .find(|te| matches!(te, TemporaryEffect::IgnoreCostThresholds { affected_cards, for_player, .. } if affected_cards.matches(card.get_id(), self) && for_player == player_id))
            .is_some();

        let mut thresholds_diff = ThresholdsDiff::default();
        if ignore_thresholds {
            thresholds_diff = card.get_costs(self)?.thresholds_cost().into();
            thresholds_diff = thresholds_diff.negate();
        }

        Ok(base_costs
            .with_mana_adjusted(total_mana_diff)
            .with_thresholds_adjusted(thresholds_diff))
    }

    pub fn queue(&mut self, effects: impl IntoIterator<Item = Effect>) {
        self.effects.extend(effects);
    }

    pub fn queue_one(&mut self, effect: Effect) {
        self.effects.push_back(effect);
    }

    pub fn queue_front(&mut self, effect: Effect) {
        self.effects.push_front(effect);
    }

    pub fn effect_log(&self) -> &[LoggedEffect] {
        self.effects.log()
    }

    pub fn effect_log_mut(&mut self) -> &mut Vec<LoggedEffect> {
        self.effects.log_mut()
    }

    pub fn temporary_effects(&self) -> &[TemporaryEffect] {
        self.effects.temporary()
    }

    pub fn temporary_effects_mut(&mut self) -> &mut Vec<TemporaryEffect> {
        self.effects.temporary_mut()
    }

    pub fn deferred_effects(&self) -> &[DeferredEffect] {
        self.effects.deferred()
    }

    pub fn deferred_effects_mut(&mut self) -> &mut Vec<DeferredEffect> {
        self.effects.deferred_mut()
    }

    pub fn get_thresholds_for_player(&self, player_id: &PlayerId) -> Thresholds {
        self.cards
            .values()
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| &c.get_controller_id(self) == player_id)
            .filter_map(|c| c.get_resource_provider())
            .map(|c| c.provided_affinity(self).unwrap_or_default())
            .sum()
    }

    pub fn get_body_of_water_at(&self, zone: &Zone) -> Option<Vec<Zone>> {
        let bodies_of_water = self.get_bodies_of_water();
        bodies_of_water
            .into_iter()
            .find(|body| body.iter().any(|z| z == zone))
    }

    pub fn get_bodies_of_water(&self) -> Vec<Vec<Zone>> {
        let mut visited: Vec<Zone> = Vec::new();
        let mut bodies_of_water: Vec<Vec<Zone>> = Vec::new();

        fn dfs(state: &State, zone: &Zone, visited: &mut Vec<Zone>, body_of_water: &mut Vec<Zone>) {
            if visited.iter().any(|z| z == zone) {
                return;
            }
            visited.push(zone.clone());

            if let Some(site) = zone.get_site(state) {
                let is_water = site.provided_affinity(state).unwrap_or_default().water > 0;
                if is_water {
                    if !body_of_water.iter().any(|z| z == zone) {
                        body_of_water.push(zone.clone());
                    }
                    for adj in zone.get_adjacent() {
                        dfs(state, &adj, visited, body_of_water);
                    }
                }
            }
        }

        for card in self
            .cards
            .values()
            .filter(|c| c.get_card_type() == CardType::Site)
        {
            let zone = card.get_zone();
            if !zone.is_in_play() {
                continue;
            }

            if let Some(site) = zone.get_site(self) {
                let is_water = site.provided_affinity(self).unwrap_or_default().water > 0;
                if is_water && !visited.iter().any(|z| z == zone) {
                    let mut body_of_water: Vec<Zone> = Vec::new();
                    dfs(self, zone, &mut visited, &mut body_of_water);
                    if !body_of_water.is_empty() {
                        bodies_of_water.push(body_of_water);
                    }
                }
            }
        }

        bodies_of_water
    }

    pub fn get_spans_of_land(&self) -> Vec<Vec<Zone>> {
        let mut visited: Vec<Zone> = Vec::new();
        let mut spans_of_land: Vec<Vec<Zone>> = Vec::new();

        fn dfs(state: &State, zone: &Zone, visited: &mut Vec<Zone>, span: &mut Vec<Zone>) {
            if visited.iter().any(|z| z == zone) {
                return;
            }
            visited.push(zone.clone());

            if let Some(site) = zone.get_site(state) {
                let is_land = site.provided_affinity(state).unwrap_or_default().water == 0;
                if is_land {
                    if !span.iter().any(|z| z == zone) {
                        span.push(zone.clone());
                    }
                    for adj in zone.get_adjacent() {
                        dfs(state, &adj, visited, span);
                    }
                }
            }
        }

        for card in self
            .cards
            .values()
            .filter(|c| c.get_card_type() == CardType::Site)
        {
            let zone = card.get_zone();
            if !zone.is_in_play() {
                continue;
            }

            if let Some(site) = zone.get_site(self) {
                let is_land = site.provided_affinity(self).unwrap_or_default().water == 0;
                if is_land && !visited.iter().any(|z| z == zone) {
                    let mut span: Vec<Zone> = Vec::new();
                    dfs(self, zone, &mut visited, &mut span);
                    if !span.is_empty() {
                        spans_of_land.push(span);
                    }
                }
            }
        }

        spans_of_land
    }

    pub fn get_body_of_water_size(&self, zone: &Zone) -> u16 {
        let mut visited: Vec<Zone> = Vec::new();
        let mut water_zones: Vec<Zone> = Vec::new();

        fn dfs(state: &State, zone: &Zone, visited: &mut Vec<Zone>, water_zones: &mut Vec<Zone>) {
            if visited.iter().any(|z| z == zone) {
                return;
            }
            visited.push(zone.clone());
            let water_site_in_zone = CardQuery::new().water_sites().in_zone(zone).any(state);
            if water_site_in_zone {
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

        for card in self.cards.values() {
            if !card.get_zone().is_in_play() {
                continue;
            }

            let card_world_effects = card.get_continuous_effects(self).await?;
            for effect in card_world_effects {
                self.continuous_effects.push(effect);
            }
        }

        Ok(())
    }

    pub fn get_player_avatar_id(&self, player_id: &PlayerId) -> anyhow::Result<uuid::Uuid> {
        self.decks
            .get(player_id)
            .map(|d| d.avatar)
            .ok_or(anyhow::anyhow!("failed to get player avatar id"))
    }

    pub fn get_opponent_id(&self, player_id: &PlayerId) -> anyhow::Result<PlayerId> {
        for player in &self.players {
            if &player.id != player_id {
                return Ok(player.id);
            }
        }

        Err(anyhow::anyhow!("failed to get opponent id"))
    }

    pub fn get_defenders_for_attack(&self, defender_id: &uuid::Uuid) -> Vec<uuid::Uuid> {
        let defender = self.get_card(defender_id);
        CardQuery::new()
            .units()
            .near_to(defender.get_zone())
            .without_ability(&Ability::CannotDefend)
            .untapped()
            .id_not(defender_id)
            .controlled_by(&defender.get_controller_id(self))
            .all(self)
    }

    pub fn get_interceptors_for_move(
        &self,
        path: &[Zone],
        controller_id: &PlayerId,
    ) -> Vec<(uuid::Uuid, Zone)> {
        let mut interceptors = Vec::new();

        for card in self.cards.values() {
            if &card.get_controller_id(self) != controller_id {
                continue;
            }
            if !card.is_unit() {
                continue;
            }
            if !card.get_zone().is_in_play() {
                continue;
            }

            let unit_zone = card.get_zone();

            let reachable_path_zones: Vec<Zone> = if card.has_ability(self, &Ability::Airborne) {
                let nearby = unit_zone.get_nearby();
                path.iter()
                    .filter(|z| nearby.contains(z) || z == &unit_zone)
                    .cloned()
                    .collect()
            } else if card.has_ability(self, &Ability::Voidwalk)
                || card
                    .get_unit_base()
                    .is_some_and(|ub| ub.abilities.iter().any(|a| matches!(a, Ability::Ranged(_))))
            {
                let adjacent = unit_zone.get_adjacent();
                path.iter()
                    .filter(|z| adjacent.contains(z) || z == &unit_zone)
                    .cloned()
                    .collect()
            } else {
                path.iter().filter(|z| z == &unit_zone).cloned().collect()
            };

            for zone in reachable_path_zones {
                interceptors.push((*card.get_id(), zone));
            }
        }

        interceptors
    }

    pub async fn apply_effects_without_log(&mut self) -> anyhow::Result<()> {
        EffectEngine::drain_without_log(self).await
    }

    pub fn data_from_cards(&self) -> Vec<CardData> {
        self.cards
            .values()
            .map(|c| CardData {
                id: *c.get_id(),
                name: c.get_name().to_string(),
                owner_id: *c.get_owner_id(),
                controller_id: c.get_controller_id(self),
                tapped: c.is_tapped(),
                edition: c.get_edition().clone(),
                zone: c.get_zone().clone(),
                card_type: c.get_card_type().clone(),
                abilities: c.get_abilities(self).unwrap_or_default(),
                region: c.get_region(self).clone(),
                damage_taken: c.get_damage_taken().unwrap_or(0),
                bearer: c.get_bearer_id().unwrap_or_default(),
                rarity: c.get_base().rarity.clone(),
                power: c.get_power(self).unwrap_or_default().unwrap_or_default(),
                has_attachments: c.has_attachments(self).unwrap_or_default(),
                image_path: c.get_image_path(),
                is_token: c.get_base().is_token,
            })
            .collect()
    }

    pub fn into_sync(&self) -> anyhow::Result<ServerMessage> {
        let mut health = HashMap::new();
        for player in &self.players {
            let avatar_id = self.get_player_avatar_id(&player.id)?;
            let avatar_card = self.get_card(&avatar_id);
            health.insert(
                player.id,
                avatar_card
                    .get_unit_base()
                    .ok_or(anyhow::anyhow!("no unit base in avatar"))?
                    .toughness
                    .saturating_sub(avatar_card.get_damage_taken().unwrap_or(0)),
            );
        }

        Ok(ServerMessage::Sync {
            cards: self.data_from_cards(),
            resources: self
                .players
                .iter()
                .map(|p| (p.id, self.get_player_resources(&p.id).unwrap().clone()))
                .collect(),
            current_player: self.current_turn_controller(),
            turn_player: self.current_player(),
            health,
            evaluation: None,
        })
    }

    pub fn get_receiver(&self) -> Receiver<ClientMessage> {
        self.client_rx.clone()
    }

    pub fn get_sender(&self) -> Sender<ServerMessage> {
        self.server_tx.clone()
    }

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> &mut dyn Card {
        &mut **self.cards.get_mut(card_id).expect("card to exist")
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> &dyn Card {
        &**self.cards.get(card_id).expect("card to exist")
    }

    pub fn get_player(&self, player_id: &PlayerId) -> anyhow::Result<&Player> {
        self.players
            .iter()
            .find(|p| &p.id == player_id)
            .ok_or(anyhow::anyhow!("failed to get player deck"))
    }

    pub fn get_player_deck_mut(&mut self, player_id: &PlayerId) -> anyhow::Result<&mut Deck> {
        self.decks
            .get_mut(player_id)
            .ok_or(anyhow::anyhow!("failed to get player deck"))
    }

    pub fn get_player_deck(&self, player_id: &PlayerId) -> anyhow::Result<&Deck> {
        self.decks
            .get(player_id)
            .ok_or(anyhow::anyhow!("failed to get player deck"))
    }

    pub fn get_player_resources(&self, player_id: &PlayerId) -> anyhow::Result<Resources> {
        Ok(Resources {
            mana: self.player_mana.get(player_id).cloned().unwrap_or(0),
            thresholds: self.get_thresholds_for_player(player_id),
        })
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub fn new_mock_state(zones_with_sites: impl AsRef<[u8]>) -> State {
        use crate::card::{AridDesert, Sorcerer, from_name_and_zone};

        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let avatar_one = Sorcerer::new(player_one_id);
        let avatar_one_id = *avatar_one.get_id();
        let avatar_two = Sorcerer::new(player_two_id);
        let avatar_two_id = *avatar_two.get_id();
        let mut cards: Vec<Box<dyn Card>> = zones_with_sites
            .as_ref()
            .iter()
            .map(|z| {
                use crate::card::Region;

                from_name_and_zone(
                    AridDesert::NAME,
                    &player_one_id,
                    Zone::Location(*z, Region::Surface),
                )
            })
            .collect();
        cards.push(Box::new(avatar_one));
        cards.push(Box::new(avatar_two));

        let player1 = PlayerWithDeck {
            player: Player {
                id: player_one_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_one_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_one_id,
            ),
            cards,
        };
        let player2 = PlayerWithDeck {
            player: Player {
                id: player_two_id,
                name: "Player 1".to_string(),
            },
            deck: Deck::new(
                &player_two_id,
                "Test Deck".to_string(),
                vec![],
                vec![],
                avatar_two_id,
            ),
            cards: vec![],
        };

        let players = vec![player1, player2];
        let (server_tx, _) = async_channel::unbounded();
        let (_, client_rx) = async_channel::unbounded();
        State::new(uuid::Uuid::new_v4(), players, server_tx, client_rx)
    }
}
