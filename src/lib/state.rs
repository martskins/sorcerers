use crate::{
    card::{
        Ability, ArtifactType, Card, CardData, CardType, Costs, DodgeRoll, MinionType, Rarity,
        Region, SiteType, Zone,
    },
    deck::Deck,
    effect::Effect,
    game::{
        Element, InputStatus, PlayerId, Resources, Thresholds, pick_card, pick_card_with_options,
        pick_zone, yes_or_no,
    },
    networking::message::{ClientMessage, ServerMessage},
    query::{EffectQuery, ZoneQuery},
};
use async_channel::{Receiver, Sender};
use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    future::Future,
    pin::Pin,
    sync::Arc,
};

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

#[derive(Debug, Default, Clone)]
pub struct CardQuery {
    id: uuid::Uuid,
    carried_by: Option<uuid::Uuid>,
    randomise: Option<bool>,
    count: Option<usize>,
    ids: Option<Vec<uuid::Uuid>>,
    card_names: Option<Vec<String>>,
    card_name_contains: Option<String>,
    not_named: Option<Vec<String>>,
    controller_id: Option<PlayerId>,
    not_in_ids: Option<Vec<uuid::Uuid>>,
    abilities: Option<Vec<Ability>>,
    without_abilities: Option<Vec<Ability>>,
    can_cast: Option<uuid::Uuid>,
    card_types: Option<Vec<CardType>>,
    minion_types: Option<Vec<MinionType>>,
    artifact_types: Option<Vec<ArtifactType>>,
    rarity: Option<Rarity>,
    mana_cost: Option<u8>,
    site_types: Option<Vec<SiteType>>,
    site_is_water: Option<bool>,
    with_affinity: Option<Vec<Element>>,
    with_affinity_in: Option<Vec<Element>>,
    in_zones: Option<Vec<Zone>>,
    within_range_of: Option<uuid::Uuid>,
    can_be_attacked_by: Option<uuid::Uuid>,
    in_regions: Option<Vec<Region>>,
    tapped: Option<bool>,
    oversized: Option<bool>,
    include_not_in_play: Option<bool>,
    can_be_targeted_by_player: Option<uuid::Uuid>,
    elements: Option<Vec<Element>>,
    prompt: Option<String>,
}

impl CardQuery {
    pub fn from_ids(ids: Vec<uuid::Uuid>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            ids: Some(ids),
            ..Default::default()
        }
    }

    pub fn from_id(id: uuid::Uuid) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            ids: Some(vec![id]),
            ..Default::default()
        }
    }

    pub fn is_randomised(&self) -> bool {
        self.randomise.unwrap_or_default()
    }

    pub fn carried_by(self, carrier_id: &uuid::Uuid) -> Self {
        Self {
            carried_by: Some(*carrier_id),
            ..self
        }
    }

    pub fn count(self, count: usize) -> Self {
        Self {
            count: Some(count),
            ..self
        }
    }

    pub fn randomised(self) -> Self {
        Self {
            randomise: Some(true),
            ..self
        }
    }

    pub async fn pick(
        &self,
        player_id: &PlayerId,
        state: &State,
        use_preview: bool,
    ) -> anyhow::Result<Option<uuid::Uuid>> {
        use crate::query::QueryCache;

        if let Some(cached) = QueryCache::matcher_results(&self.id).await {
            return Ok(Some(
                *cached
                    .first()
                    .expect("Expected at least one card to be returned from cache"),
            ));
        }

        if let Some(count) = &self.count
            && *count != 1
        {
            return Err(anyhow::anyhow!("resolve_one can only be used with count 1"));
        }

        let mut card_ids = self.all(state);
        if card_ids.is_empty() {
            return Ok(None);
        }

        // Apply must-target restrictions from cards in play (e.g. Blasted Oak)
        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            if let Some(restricted) = card.restrict_card_query_targets(state, self, &card_ids) {
                card_ids = restricted;
                break;
            }
        }
        if card_ids.is_empty() {
            return Ok(None);
        }

        let output = if let Some(true) = self.randomise {
            for card in &state.cards {
                if &card.get_controller_id(state) != player_id {
                    continue;
                }

                if let Some(query) = card.card_query_override(state, self).await? {
                    let output = Box::pin(query.pick(player_id, state, use_preview)).await?;

                    QueryCache::store_matcher_results(
                        state.game_id,
                        self.id,
                        output.map_or(vec![], |o| vec![o]),
                    )
                    .await;
                    return Ok(output);
                }
            }

            let mut rng = rand::rng();
            card_ids.shuffle(&mut rng);
            *card_ids
                .first()
                .expect("Expected at least one card to be returned from resolve_ids")
        } else {
            let prompt = self
                .prompt
                .clone()
                .unwrap_or_else(|| "Pick a card".to_string());
            if use_preview {
                pick_card_with_options(player_id, &card_ids, &card_ids, false, state, &prompt)
                    .await?
            } else {
                pick_card(player_id, &card_ids, state, &prompt).await?
            }
        };

        QueryCache::store_matcher_results(state.game_id, self.id, vec![output]).await;

        Ok(Some(output))
    }

    pub fn iter<'b>(&'b self, state: &'b State) -> impl Iterator<Item = &'b Box<dyn Card>> {
        state
            .cards
            .iter()
            .filter(|c| self.matches(c.get_id(), state))
    }

    pub fn any(&self, state: &State) -> bool {
        state.cards.iter().any(|c| self.matches(c.get_id(), state))
    }

    pub fn first(&self, state: &State) -> Option<uuid::Uuid> {
        state
            .cards
            .iter()
            .find(|c| self.matches(c.get_id(), state))
            .map(|c| *c.get_id())
    }

    pub fn all(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| self.matches(c.get_id(), state))
            .map(|c| *c.get_id())
            .collect()
    }

    pub fn with_prompt(self, prompt: &str) -> Self {
        Self {
            prompt: Some(prompt.to_string()),
            ..self
        }
    }

    pub fn card_name_contains(self, name: &str) -> Self {
        Self {
            card_name_contains: Some(name.to_string()),
            ..self
        }
    }

    pub fn not_named(self, name: &str) -> Self {
        Self {
            not_named: Some(vec![name.to_string()]),
            ..self
        }
    }

    pub fn cards_named(self, name: &str) -> Self {
        Self {
            card_names: Some(vec![name.to_string()]),
            ..self
        }
    }

    pub fn cards_with_names(self, names: Vec<String>) -> Self {
        Self {
            card_names: Some(names),
            ..self
        }
    }

    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            ..Default::default()
        }
    }

    pub fn in_play(self) -> Self {
        Self {
            in_zones: Some(Zone::all_realm()),
            include_not_in_play: Some(false),
            ..self
        }
    }

    pub fn in_zones(self, zones: &[Zone]) -> Self {
        Self {
            in_zones: Some(zones.to_vec()),
            include_not_in_play: Some(true),
            ..self
        }
    }

    pub fn normal_sized(self) -> Self {
        Self {
            oversized: Some(false),
            ..self
        }
    }

    pub fn in_zone(self, zone: &Zone) -> Self {
        Self {
            in_zones: Some(vec![zone.clone()]),
            include_not_in_play: Some(true),
            ..self
        }
    }

    pub fn including_not_in_play(self) -> Self {
        Self {
            include_not_in_play: Some(true),
            ..self
        }
    }

    pub fn untapped(self) -> Self {
        Self {
            tapped: Some(false),
            ..self
        }
    }

    pub fn tapped(self) -> Self {
        Self {
            tapped: Some(true),
            ..self
        }
    }

    pub fn adjacent_to_zones(self, zones: &[Zone]) -> Self {
        let zones = zones.iter().flat_map(|z| z.get_adjacent()).collect();
        Self {
            in_zones: Some(zones),
            ..self
        }
    }

    pub fn can_be_attacked_by(self, attacker_id: &uuid::Uuid) -> Self {
        Self {
            can_be_attacked_by: Some(*attacker_id),
            ..self
        }
    }

    pub fn within_range_of(self, card_id: &uuid::Uuid) -> Self {
        Self {
            within_range_of: Some(*card_id),
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

    pub fn near_to(self, zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            in_zones: Some(zones),
            ..self
        }
    }

    pub fn with_element(self, element: Element) -> Self {
        Self {
            elements: Some(vec![element]),
            ..self
        }
    }

    pub fn in_regions(self, region: Vec<Region>) -> Self {
        Self {
            in_regions: Some(region),
            ..self
        }
    }

    pub fn in_region(self, region: &Region) -> Self {
        Self {
            in_regions: Some(vec![region.clone()]),
            ..self
        }
    }

    pub fn with_affinities(self, elements: Vec<Element>) -> Self {
        Self {
            with_affinity: Some(elements),
            ..self
        }
    }

    pub fn with_affinity_in(self, elements: Vec<Element>) -> Self {
        Self {
            with_affinity_in: Some(elements),
            ..self
        }
    }

    pub fn with_affinity(self, elements: Element) -> Self {
        Self {
            with_affinity: Some(vec![elements]),
            ..self
        }
    }

    pub fn can_cast(self, spell: &uuid::Uuid) -> Self {
        Self {
            can_cast: Some(*spell),
            ..self
        }
    }

    pub fn without_ability(self, ability: &Ability) -> Self {
        Self {
            without_abilities: Some(vec![ability.clone()]),
            ..self
        }
    }

    pub fn with_abilities(self, abilities: Vec<Ability>) -> Self {
        Self {
            abilities: Some(abilities),
            ..self
        }
    }

    pub fn controlled_by(self, controller_id: &PlayerId) -> Self {
        Self {
            controller_id: Some(*controller_id),
            ..self
        }
    }

    pub fn id_not(self, id: &uuid::Uuid) -> Self {
        Self {
            not_in_ids: Some(vec![*id]),
            ..self
        }
    }

    pub fn id_not_in(self, not_in_ids: Vec<uuid::Uuid>) -> Self {
        Self {
            not_in_ids: Some(not_in_ids),
            ..self
        }
    }

    pub fn land_sites(self) -> Self {
        Self {
            site_is_water: Some(false),
            ..self
        }
    }

    pub fn water_sites(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Site]),
            site_is_water: Some(true),
            ..self
        }
    }

    pub fn site_types(self, site_types: Vec<SiteType>) -> Self {
        Self {
            site_types: Some(site_types),
            ..self
        }
    }

    pub fn artifacts(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Artifact]),
            ..self
        }
    }

    pub fn auras(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Aura]),
            ..self
        }
    }

    pub fn sites(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Site]),
            ..self
        }
    }

    pub fn avatars(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Avatar]),
            ..self
        }
    }

    pub fn dead(self) -> Self {
        Self {
            in_zones: Some(vec![Zone::Cemetery]),
            include_not_in_play: Some(true),
            ..self
        }
    }

    pub fn minions(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Minion]),
            ..self
        }
    }

    pub fn can_be_targeted_by_player(self, player_id: &uuid::Uuid) -> Self {
        Self {
            can_be_targeted_by_player: Some(*player_id),
            ..self
        }
    }

    pub fn units(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Minion, CardType::Avatar]),
            ..self
        }
    }

    pub fn card_types(self, card_types: Vec<CardType>) -> Self {
        Self {
            card_types: Some(card_types),
            ..self
        }
    }

    pub fn mana_cost_less_than_or_equal_to(self, mc: u8) -> Self {
        Self {
            mana_cost: Some(mc),
            ..self
        }
    }

    pub fn artifact_type(self, artifact_type: ArtifactType) -> Self {
        Self {
            artifact_types: Some(vec![artifact_type]),
            ..self
        }
    }

    pub fn artifact_types(self, artifact_types: Vec<ArtifactType>) -> Self {
        Self {
            artifact_types: Some(artifact_types),
            ..self
        }
    }

    pub fn minion_type(self, minion_types: &MinionType) -> Self {
        Self {
            minion_types: Some(vec![minion_types.clone()]),
            ..self
        }
    }

    pub fn minion_types(self, minion_types: Vec<MinionType>) -> Self {
        Self {
            minion_types: Some(minion_types),
            ..self
        }
    }

    pub fn rarity(self, rarity: &Rarity) -> Self {
        Self {
            rarity: Some(rarity.clone()),
            ..self
        }
    }

    pub fn matches(&self, card_id: &uuid::Uuid, state: &State) -> bool {
        // Cheap ID based filters
        if let Some(ids) = &self.ids
            && !ids.contains(card_id)
        {
            return false;
        }

        if let Some(not_in_ids) = &self.not_in_ids
            && not_in_ids.contains(card_id)
        {
            return false;
        }

        let card = state.get_card(card_id);
        // Zone and visibility filters
        if !self.include_not_in_play.unwrap_or_default() && !card.get_zone().is_in_play() {
            return false;
        }

        if let Some(in_zones) = &self.in_zones
            && !in_zones.iter().any(|z| card.occupies_zone(state, z))
        {
            return false;
        }

        // Simple property filters
        if let Some(controller_id) = &self.controller_id
            && &card.get_controller_id(state) != controller_id
        {
            return false;
        }

        if let Some(card_types) = &self.card_types
            && !card_types.contains(&card.get_card_type())
        {
            return false;
        }

        if let Some(tapped) = &self.tapped
            && &card.is_tapped() != tapped
        {
            return false;
        }

        if let Some(rarity) = &self.rarity
            && &card.get_base().rarity != rarity
        {
            return false;
        }

        if let Some(oversized) = &self.oversized
            && &card.is_oversized(state) != oversized
        {
            return false;
        }

        if let Some(carrier_id) = &self.carried_by
            && card.get_base().bearer.as_ref() != Some(carrier_id)
        {
            return false;
        }

        if let Some(regions) = &self.in_regions
            && !regions.contains(card.get_region(state))
        {
            return false;
        }

        // Name filters
        if let Some(name) = &self.card_name_contains
            && !card.get_name().contains(name)
        {
            return false;
        }

        if let Some(not_named) = &self.not_named
            && not_named.iter().any(|n| n == card.get_name())
        {
            return false;
        }

        if let Some(names) = &self.card_names
            && !names.iter().any(|n| n == card.get_name())
        {
            return false;
        }

        // Complex/Computed filters
        if let Some(mc) = &self.mana_cost
            && let Ok(costs) = card.get_costs(state)
            && costs.mana_value() > *mc
        {
            return false;
        }

        if let Some(elements) = &self.elements {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !elements.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(with_affinity_in) = &self.with_affinity_in {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !with_affinity_in.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(with_affinity) = &self.with_affinity {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !with_affinity.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(abilities) = &self.without_abilities {
            for ability in abilities {
                if card.has_ability(state, ability) {
                    return false;
                }
            }
        }

        if let Some(abilities) = &self.abilities {
            for ability in abilities {
                if !card.has_ability(state, ability) {
                    return false;
                }
            }
        }

        if let Some(is_water) = &self.site_is_water {
            match is_water {
                true => {
                    if let Some(site) = card.get_site() {
                        if site.provided_affinity(state).unwrap_or_default().water == 0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                false => {
                    if let Some(site) = card.get_site() {
                        if site.provided_affinity(state).unwrap_or_default().water != 0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        if let Some(site_types) = &self.site_types {
            if let Some(base) = card.get_site_base() {
                let types = &base.types;
                if !site_types.iter().any(|st| types.contains(st)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(artifact_types) = &self.artifact_types {
            if let Some(base) = card.get_artifact_base() {
                let types = &base.types;
                if !artifact_types.iter().any(|at| types.contains(at)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(minion_types) = &self.minion_types {
            if let Some(base) = card.get_unit_base() {
                let types = &base.types;
                if !minion_types.iter().any(|mt| types.contains(mt)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Very expensive filters (Cross-card or dynamic)
        if let Some(within_range_of) = &self.within_range_of {
            let other_card = state.get_card(within_range_of);
            let other_zones = other_card.zones_in_range(state);
            if !other_zones.contains(card.get_zone()) {
                return false;
            }
        }

        if let Some(spell) = &self.can_cast
            && !card
                .can_cast_spell_with_id(state, spell)
                .unwrap_or_default()
        {
            return false;
        }

        if let Some(player_id) = &self.can_be_targeted_by_player
            && !card.can_be_targetted_by_player(state, player_id)
        {
            return false;
        }

        if let Some(attacked_by) = &self.can_be_attacked_by {
            let attacker = state.get_card(attacked_by);
            if !attacker
                .get_valid_attack_targets(state, false)
                .contains(card_id)
            {
                return false;
            }
        }

        true
    }
}

pub type EffectCallback = Arc<
    dyn Sync
        + Send
        + for<'a> Fn(
            &'a State,
            &'a uuid::Uuid,
            &'a Effect,
        )
            -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + 'a>>,
>;

#[derive(Clone)]
pub struct DeferredEffect {
    pub trigger_on_effect: EffectQuery,
    pub expires_on_effect: Option<EffectQuery>,
    pub on_effect: EffectCallback,
    pub multitrigger: bool,
}

impl std::fmt::Debug for DeferredEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredEffect")
            .field("trigger_on_effect", &self.trigger_on_effect)
            .field("expires_on_effect", &self.expires_on_effect)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum TemporaryEffect {
    FloodSites {
        affected_sites: CardQuery,
        expires_on_effect: EffectQuery,
    },
    MakePlayable {
        affected_cards: CardQuery,
        expires_on_effect: EffectQuery,
    },
}

impl TemporaryEffect {
    pub fn affected_cards(&self, state: &State) -> Vec<uuid::Uuid> {
        match self {
            TemporaryEffect::FloodSites { affected_sites, .. } => affected_sites.all(state),
            // MakePlayable should always include cards not in play, so we need to add those to the
            // results of all().
            TemporaryEffect::MakePlayable { affected_cards, .. } => {
                affected_cards.clone().including_not_in_play().all(state)
            }
        }
    }

    pub fn expires_on_effect(&self) -> Option<&EffectQuery> {
        match self {
            TemporaryEffect::FloodSites {
                expires_on_effect, ..
            } => Some(expires_on_effect),
            TemporaryEffect::MakePlayable {
                expires_on_effect, ..
            } => Some(expires_on_effect),
        }
    }
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
    DoubleDamageTaken {
        affected_cards: CardQuery,
        except_strikes: bool,
    },
    ChangeSiteType {
        site_type: SiteType,
        affected_sites: CardQuery,
    },
    GrantAbility {
        ability: Ability,
        affected_cards: CardQuery,
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
pub struct LoggedEffect {
    pub effect: Arc<Effect>,
    pub turn: usize,
}

impl LoggedEffect {
    pub fn new(effect: Arc<Effect>, turn: usize) -> Self {
        Self { effect, turn }
    }
}

#[derive(Debug)]
pub struct State {
    pub game_id: uuid::Uuid,
    pub players: Vec<Player>,
    pub turns: usize,
    pub cards: Vec<Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub input_status: InputStatus,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub current_player: PlayerId,
    pub effects: VecDeque<Arc<Effect>>,
    pub effect_log: Vec<LoggedEffect>,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
    pub continuous_effects: Vec<ContinuousEffect>,
    pub temporary_effects: Vec<TemporaryEffect>,
    pub deferred_effects: Vec<DeferredEffect>,
    pub player_mana: HashMap<PlayerId, u8>,
    pub loosers: HashSet<PlayerId>,
    pub players_with_accepted_hands: HashSet<PlayerId>,
    /// Card IDs whose activated abilities have been permanently disabled (e.g. Frontier Settlers).
    pub permanently_disabled_abilities: HashSet<uuid::Uuid>,
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
        for player in players_with_decks {
            cards.extend(player.cards);
            decks.insert(player.player.id, player.deck);
        }

        State {
            game_id,
            players,
            cards,
            decks,
            turns: 0,
            input_status: InputStatus::None,
            phase: Phase::Mulligan,
            current_player: player_one,
            waiting_for_input: false,
            effects: VecDeque::new(),
            effect_log: Vec::new(),
            player_one,
            server_tx,
            client_rx,
            continuous_effects: Vec::new(),
            temporary_effects: Vec::new(),
            deferred_effects: Vec::new(),
            player_mana,
            loosers: HashSet::new(),
            players_with_accepted_hands: HashSet::new(),
            permanently_disabled_abilities: HashSet::new(),
        }
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
    ) -> anyhow::Result<Costs> {
        let card = self.get_card(card_id);
        let base_costs = card.get_costs(self)?.clone();

        let total_diff: i8 = self
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

        if total_diff == 0 {
            return Ok(base_costs);
        }

        Ok(base_costs.with_mana_adjusted(total_diff))
    }

    pub fn queue(&mut self, effects: impl IntoIterator<Item = Effect>) {
        self.effects.extend(effects.into_iter().map(Arc::new));
    }

    pub fn queue_one(&mut self, effect: Effect) {
        self.effects.push_back(Arc::new(effect));
    }

    pub fn queue_front(&mut self, effect: Effect) {
        self.effects.push_front(Arc::new(effect));
    }

    pub fn get_thresholds_for_player(&self, player_id: &PlayerId) -> Thresholds {
        self.cards
            .iter()
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
            .iter()
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

        for card in &self.cards {
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

        for card in &self.cards {
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
                    - avatar_card.get_damage_taken().unwrap_or(0),
            );
        }

        Ok(ServerMessage::Sync {
            cards: self.data_from_cards(),
            resources: self
                .players
                .iter()
                .map(|p| (p.id, self.get_player_resources(&p.id).unwrap().clone()))
                .collect(),
            current_player: self.current_player,
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
        self.cards
            .iter_mut()
            .find(|c| c.get_id() == card_id)
            .expect("failed to get card")
            .as_mut()
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> &dyn Card {
        self.cards
            .iter()
            .find(|c| c.get_id() == card_id)
            .expect("failed to get card")
            .as_ref()
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

    pub fn snapshot(&self) -> State {
        State {
            game_id: self.game_id,
            players: self.players.clone(),
            cards: self.cards.iter().map(|c| c.clone_box()).collect(),
            decks: self.decks.clone(),
            turns: self.turns,
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
            temporary_effects: self.temporary_effects.clone(),
            deferred_effects: self.deferred_effects.clone(),
            player_mana: self.player_mana.clone(),
            loosers: self.loosers.clone(),
            players_with_accepted_hands: self.players_with_accepted_hands.clone(),
            permanently_disabled_abilities: self.permanently_disabled_abilities.clone(),
        }
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub fn new_mock_state(zones_with_sites: impl AsRef<[Zone]>) -> State {
        use crate::card::{AridDesert, from_name_and_zone};

        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let cards: Vec<Box<dyn Card>> = zones_with_sites
            .as_ref()
            .iter()
            .map(|z| from_name_and_zone(AridDesert::NAME, &player_one_id, z.clone()))
            .collect();

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
                uuid::Uuid::nil(),
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
                uuid::Uuid::nil(),
            ),
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
    use crate::card::{
        CauldronCrones, DonnybrookInn, HeadlessHaunt, KiteArcher, NimbusJinn, RimlandNomads,
    };

    #[test]
    fn test_inteceptors() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let mut rimland_nomads = RimlandNomads::new(player_id);
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id;
        let mut kite_archer = KiteArcher::new(opponent_id);
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
        let player_id = state.players[0].id;
        let mut rimland_nomads = RimlandNomads::new(player_id);
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id;
        let mut kite_archer = KiteArcher::new(opponent_id);
        kite_archer.set_zone(Zone::Realm(11));
        state.cards.push(Box::new(kite_archer.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 0);
    }

    #[test]
    fn test_voidwalking_interceptor() {
        let mut state =
            State::new_mock_state(vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)]);
        let player_id = state.players[0].id;
        let mut rimland_nomads = RimlandNomads::new(player_id);
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id;
        let mut headless_haunt = HeadlessHaunt::new(opponent_id);
        headless_haunt.set_zone(Zone::Realm(12));
        state.cards.push(Box::new(headless_haunt.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 1);
    }

    #[test]
    fn test_airborne_interceptor() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let mut rimland_nomads = RimlandNomads::new(player_id);
        rimland_nomads.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(rimland_nomads.clone()));

        let opponent_id = state.players[1].id;
        let mut headless_haunt = NimbusJinn::new(opponent_id);
        headless_haunt.set_zone(Zone::Realm(12));
        state.cards.push(Box::new(headless_haunt.clone()));

        let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
        let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
        assert_eq!(interceptors.len(), 3);
    }

    #[tokio::test]
    async fn test_get_effective_costs_donnybrook_inn() {
        let mut state = State::new_mock_state(vec![Zone::Realm(8)]);
        let player_id = state.players[0].id;
        let mut donnybrook_inn = DonnybrookInn::new(player_id);
        donnybrook_inn.set_zone(Zone::Realm(3));
        state.cards.push(Box::new(donnybrook_inn.clone()));

        let mut cauldron_crones = CauldronCrones::new(player_id);
        cauldron_crones.set_zone(Zone::Hand);
        state.cards.push(Box::new(cauldron_crones.clone()));

        state.compute_world_effects().await.unwrap();
        let regular_costs = state
            .get_effective_costs(cauldron_crones.get_id(), None)
            .unwrap();
        assert_eq!(regular_costs.mana_value(), 3);

        let inn_costs = state
            .get_effective_costs(cauldron_crones.get_id(), Some(donnybrook_inn.get_zone()))
            .unwrap();
        assert_eq!(inn_costs.mana_value(), 2);
    }
}
