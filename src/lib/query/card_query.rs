use rand::seq::IndexedRandom;

use crate::{
    card::{
        Ability, ArtifactType, Card, CardStatus, CardType, MinionType, Rarity, Region, SiteType,
    },
    game::{CardId, Direction, Element, PlayerId, pick_card_source, pick_card_with_options_source},
    state::State,
    zone::{Location, Zone},
};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone)]
pub struct CardQuery {
    id: Arc<OnceLock<uuid::Uuid>>,
    carried_by: Option<Option<CardId>>,
    randomise: Option<bool>,
    count: Option<usize>,
    card_id: Option<Vec<ValueFilter<CardId>>>,
    card_name: Option<Vec<StringFilter>>,
    controller_id: Option<PlayerId>,
    same_controller_as: Option<CardId>,
    different_controller_than: Option<CardId>,
    abilities: Option<Vec<VecFilter<Ability>>>,
    statuses: Option<Vec<VecFilter<CardStatus>>>,
    card_types: Option<Vec<CardType>>,
    minion_types: Option<Vec<MinionType>>,
    artifact_types: Option<Vec<ArtifactType>>,
    rarity: Option<Rarity>,
    mana_cost: Option<NumericFilter<u8>>,
    power: Option<NumericFilter<u16>>,
    site_types: Option<Vec<SiteType>>,
    site_is_water: Option<bool>,
    with_affinity: Option<Vec<Element>>,
    with_affinity_in: Option<Vec<Element>>,
    in_zones: Option<Vec<Zone>>,
    regions: Option<Vec<Region>>,
    within_range_of: Option<CardId>,
    can_be_attacked_by: Option<CardId>,
    tapped: Option<bool>,
    oversized: Option<bool>,
    include_not_in_play: Option<bool>,
    can_be_targeted_by_player: Option<CardId>,
    elements: Option<Vec<Element>>,
    spatial_filters: Vec<SpatialFilter>,
    prompt: Option<String>,
    source_card_id: Option<CardId>,
    allow_modifiers: bool,
    bearer_of: Option<CardId>,
}

impl Default for CardQuery {
    fn default() -> Self {
        Self {
            id: Arc::new(OnceLock::new()),
            carried_by: None,
            randomise: None,
            count: None,
            card_id: None,
            card_name: None,
            controller_id: None,
            same_controller_as: None,
            different_controller_than: None,
            abilities: None,
            statuses: None,
            card_types: None,
            minion_types: None,
            artifact_types: None,
            rarity: None,
            mana_cost: None,
            power: None,
            site_types: None,
            site_is_water: None,
            with_affinity: None,
            with_affinity_in: None,
            in_zones: None,
            regions: None,
            within_range_of: None,
            can_be_attacked_by: None,
            tapped: None,
            oversized: None,
            include_not_in_play: None,
            can_be_targeted_by_player: None,
            elements: None,
            spatial_filters: Vec::new(),
            prompt: None,
            source_card_id: None,
            allow_modifiers: true,
            bearer_of: None,
        }
    }
}

#[derive(Debug, Clone)]
enum StringFilter {
    OneOf(Vec<String>),
    Equals(String),
    NotEquals(String),
    ContainsSubstr(String),
}

impl StringFilter {
    fn matches(&self, val: &str) -> bool {
        match self {
            StringFilter::OneOf(items) => items.contains(&val.to_string()),
            StringFilter::Equals(item) => item == val,
            StringFilter::NotEquals(item) => item != val,
            StringFilter::ContainsSubstr(substr) => val.contains(substr),
        }
    }
}

#[derive(Debug, Clone)]
enum ValueFilter<T> {
    OneOf(Vec<T>),
    NoneOf(Vec<T>),
    Equals(T),
    NotEquals(T),
}

impl<T: PartialEq> ValueFilter<T> {
    fn matches(&self, val: &T) -> bool {
        match self {
            ValueFilter::OneOf(items) => items.contains(val),
            ValueFilter::NoneOf(items) => !items.contains(val),
            ValueFilter::Equals(item) => item == val,
            ValueFilter::NotEquals(item) => item != val,
        }
    }
}

#[derive(Debug, Clone)]
enum VecFilter<T> {
    WithAll(Vec<T>),
    WithoutAny(Vec<T>),
    With(T),
    Without(T),
}

impl<T: PartialEq> VecFilter<T> {
    fn matches(&self, vals: &[T]) -> bool {
        match self {
            VecFilter::WithAll(items) => items.iter().all(|i| vals.contains(i)),
            VecFilter::WithoutAny(items) => items.iter().all(|i| !vals.contains(i)),
            VecFilter::With(item) => vals.contains(item),
            VecFilter::Without(item) => !vals.contains(item),
        }
    }
}

#[derive(Debug, Clone)]
enum NumericFilter<T> {
    #[allow(dead_code)]
    GreaterThan(T),
    GreaterThanOrEqualTo(T),
    LessThan(T),
    LessThanOrEqualTo(T),
    EqualTo(T),
}

impl<T: PartialOrd + PartialEq> NumericFilter<T> {
    fn matches(&self, mc: T) -> bool {
        match self {
            NumericFilter::GreaterThan(val) => mc > *val,
            NumericFilter::GreaterThanOrEqualTo(val) => mc >= *val,
            NumericFilter::LessThan(val) => mc < *val,
            NumericFilter::LessThanOrEqualTo(val) => mc <= *val,
            NumericFilter::EqualTo(val) => mc == *val,
        }
    }
}

#[derive(Debug, Clone)]
enum SpatialFilter {
    ZoneOfCard(uuid::Uuid),
    ZoneAndDirectionFromCard {
        card_id: CardId,
        direction: Direction,
        steps: u8,
        normalise_for_owner: bool,
    },
    AdjacentLocations(Zone),
    AdjacentLocationsToAny(Vec<Zone>),
    NearbyLocations(Zone),
    NearbyZonesToCard(uuid::Uuid),
    NearbyLocationsToCard(uuid::Uuid),
    AffectedZonesOfCard(uuid::Uuid),
    AdjacentSites(Zone),
    NearbySites(Zone),
    NearbySitesToCard(uuid::Uuid),
    AdjacentVoids(Zone),
    NearbyVoids(Zone),
}

struct PreparedCardQuery<'a> {
    query: &'a CardQuery,
    state: &'a State,
    spatial_zones: Vec<Vec<Zone>>,
}

impl<'a> PreparedCardQuery<'a> {
    fn new(query: &'a CardQuery, state: &'a State) -> Self {
        let spatial_zones = query
            .spatial_filters
            .iter()
            .map(|filter| match filter {
                SpatialFilter::AdjacentLocations(zone) => zone.get_adjacent_locations(state),
                SpatialFilter::AdjacentLocationsToAny(zones) => zones
                    .iter()
                    .flat_map(|zone| zone.get_adjacent_locations(state))
                    .collect(),
                SpatialFilter::NearbyLocations(zone) => zone.get_nearby_locations(state),
                SpatialFilter::ZoneOfCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| vec![card.get_zone().clone()])
                    .unwrap_or_default(),
                SpatialFilter::ZoneAndDirectionFromCard {
                    card_id,
                    direction,
                    steps,
                    normalise_for_owner,
                } => state
                    .cards
                    .get(card_id)
                    .map(|card| {
                        let board_flipped = *normalise_for_owner
                            && card.get_controller_id(state) != state.player_one;
                        let mut zones = vec![card.get_zone().clone()];
                        if let Some(zone) = card
                            .get_zone()
                            .zone_in_direction(&direction.normalise(board_flipped), *steps)
                        {
                            zones.push(zone);
                        }
                        zones
                    })
                    .unwrap_or_default(),
                SpatialFilter::NearbyZonesToCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| card.get_zone().get_nearby())
                    .unwrap_or_default(),
                SpatialFilter::NearbyLocationsToCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| card.get_zone().get_nearby_locations(state))
                    .unwrap_or_default(),
                SpatialFilter::AffectedZonesOfCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| {
                        card.get_aura()
                            .map(|aura| aura.get_affected_zones(state))
                            .unwrap_or_else(|| vec![card.get_zone().clone()])
                    })
                    .unwrap_or_default(),
                SpatialFilter::AdjacentSites(zone) => zone.get_adjacent_sites(state),
                SpatialFilter::NearbySites(zone) => zone.get_nearby_sites(state),
                SpatialFilter::NearbySitesToCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| card.get_zone().get_nearby_sites(state))
                    .unwrap_or_default(),
                SpatialFilter::AdjacentVoids(zone) => zone.get_adjacent_voids(state),
                SpatialFilter::NearbyVoids(zone) => zone.get_nearby_voids(state),
            })
            .collect();

        Self {
            query,
            state,
            spatial_zones,
        }
    }

    fn matches_card(&self, card: &dyn Card) -> bool {
        let query = self.query;
        let state = self.state;
        let card_id = card.get_id();

        // Cheap ID based filters
        if let Some(filters) = &query.card_id {
            let card_id = card.get_id();
            for filter in filters {
                if !filter.matches(card_id) {
                    return false;
                }
            }
        }

        // Zone and visibility filters
        if !query.include_not_in_play.unwrap_or_default() && !card.get_zone().is_in_play() {
            return false;
        }

        if let Some(in_zones) = &query.in_zones
            && !in_zones.iter().any(|z| self.card_occupies_zone(card, z))
        {
            return false;
        }

        if let Some(regions) = &query.regions
            && !regions.contains(card.get_region(state))
        {
            return false;
        }

        if self
            .spatial_zones
            .iter()
            .any(|zones| !zones.iter().any(|zone| self.card_occupies_zone(card, zone)))
        {
            return false;
        }

        // Simple property filters
        if let Some(controller_id) = &query.controller_id
            && &card.get_controller_id(state) != controller_id
        {
            return false;
        }

        if let Some(source_id) = &query.same_controller_as {
            let Some(source) = state.cards.get(source_id) else {
                return false;
            };
            if card.get_controller_id(state) != source.get_controller_id(state) {
                return false;
            }
        }

        if let Some(source_id) = &query.different_controller_than {
            let Some(source) = state.cards.get(source_id) else {
                return false;
            };
            if card.get_controller_id(state) == source.get_controller_id(state) {
                return false;
            }
        }

        if let Some(card_types) = &query.card_types
            && !card_types.contains(&card.get_card_type())
            && !(card_types.contains(&CardType::Minion) && state.is_minion_card(card_id))
            && !(card_types.contains(&CardType::Avatar) && card.is_avatar())
        {
            return false;
        }

        if let Some(tapped) = &query.tapped
            && &card.is_tapped() != tapped
        {
            return false;
        }

        if let Some(rarity) = &query.rarity
            && &card.get_base().rarity != rarity
        {
            return false;
        }

        if let Some(oversized) = &query.oversized
            && &card.is_oversized(state) != oversized
        {
            return false;
        }

        if let Some(carrier_id) = &query.carried_by
            && card.get_base().bearer != *carrier_id
        {
            return false;
        }

        if let Some(source_id) = &query.bearer_of {
            let Some(source) = state.cards.get(source_id) else {
                return false;
            };
            if source.get_base().bearer != Some(*card_id) {
                return false;
            }
        }

        // Name filters
        if let Some(filters) = &query.card_name {
            let name = card.get_name();
            for filter in filters {
                if !filter.matches(name) {
                    return false;
                }
            }
        }

        // Complex/Computed filters
        if let Some(mc) = &query.mana_cost
            && let Ok(costs) = card.get_costs(state)
            && !mc.matches(costs.mana_value())
        {
            return false;
        }

        if let Some(elements) = &query.elements {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !elements.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(with_affinity_in) = &query.with_affinity_in {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !with_affinity_in.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(with_affinity) = &query.with_affinity {
            let card_elements = card.get_elements(state).unwrap_or_default();
            if !with_affinity.iter().any(|e| card_elements.contains(e)) {
                return false;
            }
        }

        if let Some(abilities) = &query.abilities {
            let card_abilities = card.get_abilities(state).unwrap_or_default();
            for filter in abilities {
                if !filter.matches(&card_abilities) {
                    return false;
                }
            }
        }

        if let Some(statuses) = &query.statuses {
            let card_statuses = card.get_statuses(state);
            for filter in statuses {
                if !filter.matches(&card_statuses) {
                    return false;
                }
            }
        }

        if let Some(is_water) = &query.site_is_water {
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

        if let Some(site_types) = &query.site_types {
            if let Some(base) = card.get_site_base() {
                let types = &base.types;
                if !site_types.iter().any(|st| types.contains(st)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(artifact_types) = &query.artifact_types {
            if let Some(base) = card.get_artifact_base() {
                let types = &base.types;
                if !artifact_types.iter().any(|at| types.contains(at)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(minion_types) = &query.minion_types {
            if let Some(base) = card.get_unit_base() {
                let types = &base.types;
                if !minion_types.iter().any(|mt| types.contains(mt)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(power) = &query.power
            && card
                .get_power(state)
                .ok()
                .flatten()
                .is_none_or(|p| !power.matches(p))
        {
            return false;
        }

        // Very expensive filters (Cross-card or dynamic)
        if let Some(within_range_of) = &query.within_range_of {
            let other_card = state.get_card(within_range_of);
            let other_zones = other_card.zones_in_range(state);
            if !other_zones.contains(card.get_zone()) {
                return false;
            }
        }

        if let Some(player_id) = &query.can_be_targeted_by_player
            && !card.can_be_targetted_by_player(state, player_id)
        {
            return false;
        }

        if let Some(attacked_by) = &query.can_be_attacked_by {
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

    fn card_occupies_zone(&self, card: &dyn Card, zone: &Zone) -> bool {
        match (zone, card.get_zone()) {
            (
                Zone::Location(Location::Square(sq, region)),
                Zone::Location(Location::Square(card_square, card_region)),
            ) => sq == card_square && region == card_region,
            (
                Zone::Location(Location::Square(sq, region)),
                Zone::Location(Location::Intersection(card_squares, card_region)),
            ) => card_squares.contains(sq) && region == card_region,
            _ => card.get_zone() == zone,
        }
    }
}

impl CardQuery {
    pub fn from_ids(ids: Vec<CardId>) -> Self {
        Self {
            card_id: Some(vec![ValueFilter::OneOf(ids)]),
            ..Default::default()
        }
    }

    pub fn from_id(id: CardId) -> Self {
        Self {
            card_id: Some(vec![ValueFilter::Equals(id)]),
            ..Default::default()
        }
    }

    pub fn is_randomised(&self) -> bool {
        self.randomise.unwrap_or_default()
    }

    pub fn not_carried(self) -> Self {
        Self {
            carried_by: Some(None),
            ..self
        }
    }

    pub fn carried_by(self, carrier_id: &uuid::Uuid) -> Self {
        Self {
            carried_by: Some(Some(*carrier_id)),
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
    ) -> anyhow::Result<Option<CardId>> {
        use crate::query::QueryCache;

        let query_id = *self.id.get_or_init(uuid::Uuid::new_v4);
        if let Some(cached) = QueryCache::card_result(&query_id) {
            return Ok(Some(cached));
        }

        if let Some(count) = &self.count
            && *count != 1
        {
            return Err(anyhow::anyhow!("resolve_one can only be used with count 1"));
        }

        let mut effective_query = self.clone();
        let mut card_ids = effective_query.all(state);
        if card_ids.is_empty() {
            return Ok(None);
        }

        if self.allow_modifiers {
            for effect in state.active_continuous_effects() {
                if let crate::state::OngoingEffect::RestrictCardTargets { restriction, .. } = effect
                    && let Some(restricted) =
                        restriction(state, player_id, &effective_query, &card_ids)
                {
                    card_ids = restricted;
                    break;
                }
            }

            if card_ids.is_empty() {
                return Ok(None);
            }

            effective_query = effective_query.id_in(card_ids.clone());

            for effect in state.active_continuous_effects() {
                if let crate::state::OngoingEffect::ModifyCardQuery { modifier, .. } = effect
                    && let Some(query) = modifier(state, player_id, &effective_query)?
                {
                    let output = Box::pin(query.without_modifiers().pick(
                        player_id,
                        state,
                        use_preview,
                    ))
                    .await?;
                    if let Some(output) = output {
                        QueryCache::store_card_result(state.game_id, query_id, output);
                    }
                    return Ok(output);
                }
            }
        }

        if card_ids.is_empty() {
            return Ok(None);
        }

        let output = if let Some(true) = effective_query.randomise {
            *card_ids
                .choose(&mut rand::rng())
                .expect("a card to be picked")
        } else {
            let prompt = effective_query
                .prompt
                .clone()
                .unwrap_or_else(|| "Pick a card".to_string());
            if use_preview {
                pick_card_with_options_source(
                    player_id,
                    &card_ids,
                    &card_ids,
                    false,
                    state,
                    &prompt,
                    effective_query.source_card_id,
                )
                .await?
            } else {
                pick_card_source(
                    player_id,
                    &card_ids,
                    state,
                    &prompt,
                    effective_query.source_card_id,
                )
                .await?
            }
        };

        QueryCache::store_card_result(state.game_id, query_id, output);

        Ok(Some(output))
    }

    pub fn iter<'b>(&'b self, state: &'b State) -> impl Iterator<Item = &'b Box<dyn Card>> {
        let prepared = PreparedCardQuery::new(self, state);
        state
            .cards
            .values()
            .filter(move |c| prepared.matches_card(c.as_ref()))
    }

    pub fn any(&self, state: &State) -> bool {
        if let Some(carrier_id) = self.only_carried_by_filter() {
            return state
                .cards
                .values()
                .any(|card| card.get_zone().is_in_play() && card.get_base().bearer == carrier_id);
        }

        let prepared = PreparedCardQuery::new(self, state);
        state
            .cards
            .values()
            .any(|c| prepared.matches_card(c.as_ref()))
    }

    pub fn first(&self, state: &State) -> Option<CardId> {
        if let Some(carrier_id) = self.only_carried_by_filter() {
            return state
                .cards
                .values()
                .find(|card| card.get_zone().is_in_play() && card.get_base().bearer == carrier_id)
                .map(|card| *card.get_id());
        }

        let prepared = PreparedCardQuery::new(self, state);
        state
            .cards
            .values()
            .find(|c| prepared.matches_card(c.as_ref()))
            .map(|c| *c.get_id())
    }

    pub fn all(&self, state: &State) -> Vec<CardId> {
        if let Some(carrier_id) = self.only_carried_by_filter() {
            return state
                .cards
                .values()
                .filter(|card| card.get_zone().is_in_play() && card.get_base().bearer == carrier_id)
                .map(|card| *card.get_id())
                .collect();
        }

        let prepared = PreparedCardQuery::new(self, state);
        state
            .cards
            .values()
            .filter(|c| prepared.matches_card(c.as_ref()))
            .map(|c| *c.get_id())
            .collect()
    }

    fn only_carried_by_filter(&self) -> Option<Option<CardId>> {
        if self.carried_by.is_some()
            && self.randomise.is_none()
            && self.count.is_none()
            && self.card_id.is_none()
            && self.card_name.is_none()
            && self.controller_id.is_none()
            && self.same_controller_as.is_none()
            && self.different_controller_than.is_none()
            && self.abilities.is_none()
            && self.statuses.is_none()
            && self.card_types.is_none()
            && self.minion_types.is_none()
            && self.artifact_types.is_none()
            && self.rarity.is_none()
            && self.mana_cost.is_none()
            && self.site_types.is_none()
            && self.site_is_water.is_none()
            && self.with_affinity.is_none()
            && self.with_affinity_in.is_none()
            && self.in_zones.is_none()
            && self.regions.is_none()
            && self.within_range_of.is_none()
            && self.can_be_attacked_by.is_none()
            && self.tapped.is_none()
            && self.oversized.is_none()
            && self.include_not_in_play.is_none()
            && self.can_be_targeted_by_player.is_none()
            && self.elements.is_none()
            && self.spatial_filters.is_empty()
            && self.bearer_of.is_none()
        {
            self.carried_by
        } else {
            None
        }
    }

    pub fn with_prompt(self, prompt: &str) -> Self {
        Self {
            prompt: Some(prompt.to_string()),
            ..self
        }
    }

    pub fn with_source_card(self, card_id: CardId) -> Self {
        Self {
            source_card_id: Some(card_id),
            ..self
        }
    }

    pub fn source_card_id(&self) -> Option<CardId> {
        self.source_card_id
    }

    fn id_in(self, ids: Vec<CardId>) -> Self {
        let mut new_filter = self.card_id.unwrap_or_default();
        new_filter.push(ValueFilter::OneOf(ids));
        Self {
            card_id: Some(new_filter),
            ..self
        }
    }

    fn without_modifiers(self) -> Self {
        Self {
            allow_modifiers: false,
            ..self
        }
    }

    pub fn name_contains(self, name: String) -> Self {
        let mut new_filter = self.card_name.unwrap_or_default();
        new_filter.push(StringFilter::ContainsSubstr(name));
        Self {
            card_name: Some(new_filter),
            ..self
        }
    }

    pub fn not_named(self, name: String) -> Self {
        let mut new_filter = self.card_name.unwrap_or_default();
        new_filter.push(StringFilter::NotEquals(name));
        Self {
            card_name: Some(new_filter),
            ..self
        }
    }

    pub fn named(self, name: String) -> Self {
        let mut new_filter = self.card_name.unwrap_or_default();
        new_filter.push(StringFilter::Equals(name));
        Self {
            card_name: Some(new_filter),
            ..self
        }
    }

    pub fn name_in(self, names: Vec<String>) -> Self {
        let mut new_filter = self.card_name.unwrap_or_default();
        new_filter.push(StringFilter::OneOf(names));
        Self {
            card_name: Some(new_filter),
            ..self
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mana_cost(self, mana_cost: u8) -> Self {
        Self {
            mana_cost: Some(NumericFilter::EqualTo(mana_cost)),
            ..self
        }
    }

    pub fn in_play(self) -> Self {
        Self {
            in_zones: Some(Zone::all_board()),
            include_not_in_play: Some(false),
            ..self
        }
    }

    pub fn in_zones(self, zones: &[Zone]) -> Self {
        Self {
            in_zones: Some(zones.to_vec()),
            ..self
        }
    }

    pub fn in_region(self, region: Region) -> Self {
        Self {
            regions: Some(vec![region]),
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
            ..self
        }
    }

    pub fn in_zone_of_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(SpatialFilter::ZoneOfCard(*card_id));
        self
    }

    pub fn in_zone_and_direction_from_card(
        mut self,
        card_id: &CardId,
        direction: Direction,
        steps: u8,
        normalise_for_owner: bool,
    ) -> Self {
        self.spatial_filters
            .push(SpatialFilter::ZoneAndDirectionFromCard {
                card_id: *card_id,
                direction,
                steps,
                normalise_for_owner,
            });
        self
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
        self.adjacent_locations_to_any(zones)
    }

    pub fn can_be_attacked_by(self, attacker_id: &uuid::Uuid) -> Self {
        Self {
            can_be_attacked_by: Some(*attacker_id),
            ..self
        }
    }

    pub fn within_range_of(self, card_id: &CardId) -> Self {
        Self {
            within_range_of: Some(*card_id),
            ..self
        }
    }

    pub fn adjacent_to(self, zone: &Zone) -> Self {
        self.adjacent_locations_to(zone)
    }

    pub fn near_to(self, zone: &Zone) -> Self {
        self.nearby_locations_to(zone)
    }

    pub fn adjacent_locations_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::AdjacentLocations(zone.clone()));
        self
    }

    pub fn adjacent_locations_to_any(mut self, zones: &[Zone]) -> Self {
        self.spatial_filters
            .push(SpatialFilter::AdjacentLocationsToAny(zones.to_vec()));
        self
    }

    pub fn nearby_locations_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbyLocations(zone.clone()));
        self
    }

    pub fn nearby_locations_to_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbyLocationsToCard(*card_id));
        self
    }

    pub fn nearby_zones_to_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbyZonesToCard(*card_id));
        self
    }

    pub fn in_affected_zones_of_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(SpatialFilter::AffectedZonesOfCard(*card_id));
        self
    }

    pub fn adjacent_sites_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::AdjacentSites(zone.clone()));
        self
    }

    pub fn nearby_sites_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbySites(zone.clone()));
        self
    }

    pub fn nearby_sites_to_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbySitesToCard(*card_id));
        self
    }

    pub fn adjacent_voids_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::AdjacentVoids(zone.clone()));
        self
    }

    pub fn nearby_voids_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbyVoids(zone.clone()));
        self
    }

    pub fn with_element(self, element: Element) -> Self {
        Self {
            elements: Some(vec![element]),
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

    pub fn without_ability(self, ability: Ability) -> Self {
        let mut new_filter = self.abilities.unwrap_or_default();
        new_filter.push(VecFilter::Without(ability));
        Self {
            abilities: Some(new_filter),
            ..self
        }
    }

    pub fn without_abilities(self, abilities: Vec<Ability>) -> Self {
        let mut new_filter = self.abilities.unwrap_or_default();
        new_filter.push(VecFilter::WithoutAny(abilities));
        Self {
            abilities: Some(new_filter),
            ..self
        }
    }

    pub fn with_ability(self, ability: Ability) -> Self {
        let mut new_filter = self.abilities.unwrap_or_default();
        new_filter.push(VecFilter::With(ability));
        Self {
            abilities: Some(new_filter),
            ..self
        }
    }

    pub fn with_abilities(self, abilities: Vec<Ability>) -> Self {
        let mut new_filter = self.abilities.unwrap_or_default();
        new_filter.push(VecFilter::WithAll(abilities));
        Self {
            abilities: Some(new_filter),
            ..self
        }
    }

    pub fn without_status(self, status: CardStatus) -> Self {
        let mut new_filter = self.statuses.unwrap_or_default();
        new_filter.push(VecFilter::Without(status));
        Self {
            statuses: Some(new_filter),
            ..self
        }
    }

    pub fn with_status(self, status: CardStatus) -> Self {
        let mut new_filter = self.statuses.unwrap_or_default();
        new_filter.push(VecFilter::With(status));
        Self {
            statuses: Some(new_filter),
            ..self
        }
    }

    pub fn controlled_by(self, controller_id: &PlayerId) -> Self {
        Self {
            controller_id: Some(*controller_id),
            ..self
        }
    }

    pub fn controlled_by_same_controller_as_card(self, card_id: &CardId) -> Self {
        Self {
            same_controller_as: Some(*card_id),
            ..self
        }
    }

    pub fn controlled_by_different_controller_than_card(self, card_id: &CardId) -> Self {
        Self {
            different_controller_than: Some(*card_id),
            ..self
        }
    }

    pub fn bearer_of_card(self, card_id: &CardId) -> Self {
        Self {
            bearer_of: Some(*card_id),
            ..self
        }
    }

    pub fn id_not(self, id: CardId) -> Self {
        let mut new_filter = self.card_id.unwrap_or_default();
        new_filter.push(ValueFilter::NotEquals(id));
        Self {
            card_id: Some(new_filter),
            ..self
        }
    }

    pub fn id_not_in(self, not_in_ids: Vec<CardId>) -> Self {
        let mut new_filter = self.card_id.unwrap_or_default();
        new_filter.push(ValueFilter::NoneOf(not_in_ids));
        Self {
            card_id: Some(new_filter),
            ..self
        }
    }

    pub fn land_sites(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Site]),
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

    pub fn magics(self) -> Self {
        Self {
            card_types: Some(vec![CardType::Magic]),
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

    pub fn mana_cost_lte(self, mc: u8) -> Self {
        Self {
            mana_cost: Some(NumericFilter::LessThanOrEqualTo(mc)),
            ..self
        }
    }

    pub fn power_gte(self, power: u16) -> Self {
        Self {
            power: Some(NumericFilter::GreaterThanOrEqualTo(power)),
            ..self
        }
    }

    pub fn power_lte(self, power: u16) -> Self {
        Self {
            power: Some(NumericFilter::LessThanOrEqualTo(power)),
            ..self
        }
    }

    pub fn power_lt(self, power: u16) -> Self {
        Self {
            power: Some(NumericFilter::LessThan(power)),
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

    pub fn matches(&self, card_id: &CardId, state: &State) -> bool {
        if let Some(filters) = &self.card_id {
            for filter in filters {
                if !filter.matches(card_id) {
                    return false;
                }
            }
        }

        let card = state.get_card(card_id);
        PreparedCardQuery::new(self, state).matches_card(card)
    }
}
