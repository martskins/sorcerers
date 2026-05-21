use rand::seq::IndexedRandom;

use crate::{
    card::{Ability, ArtifactType, Card, CardType, MinionType, Rarity, Region, SiteType},
    game::{Element, PlayerId, pick_card_source, pick_card_with_options_source},
    state::State,
    zone::Zone,
};

#[derive(Debug, Default, Clone)]
pub struct CardQuery {
    id: uuid::Uuid,
    carried_by: Option<Option<uuid::Uuid>>,
    randomise: Option<bool>,
    count: Option<usize>,
    ids: Option<Vec<uuid::Uuid>>,
    card_names: Option<Vec<String>>,
    card_name_contains: Option<String>,
    not_named: Option<Vec<String>>,
    controller_id: Option<PlayerId>,
    not_in_ids: Option<Vec<uuid::Uuid>>,
    without_abilities: Option<Vec<Ability>>,
    with_abilities: Option<Vec<Ability>>,
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
    regions: Option<Vec<Region>>,
    within_range_of: Option<uuid::Uuid>,
    can_be_attacked_by: Option<uuid::Uuid>,
    tapped: Option<bool>,
    oversized: Option<bool>,
    include_not_in_play: Option<bool>,
    can_be_targeted_by_player: Option<uuid::Uuid>,
    elements: Option<Vec<Element>>,
    spatial_filters: Vec<SpatialFilter>,
    prompt: Option<String>,
    source_card_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
enum SpatialFilter {
    AdjacentLocations(Zone),
    AdjacentLocationsToAny(Vec<Zone>),
    NearbyLocations(Zone),
    NearbyLocationsToCard(uuid::Uuid),
    AdjacentSites(Zone),
    NearbySites(Zone),
    AdjacentVoids(Zone),
    NearbyVoids(Zone),
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
    ) -> anyhow::Result<Option<uuid::Uuid>> {
        use crate::query::QueryCache;

        if let Some(cached) = QueryCache::card_result(&self.id) {
            return Ok(Some(cached));
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
        for card in state.cards.values().filter(|c| c.get_zone().is_in_play()) {
            if let Some(restricted) = card.restrict_card_query_targets(state, self, &card_ids) {
                card_ids = restricted;
                break;
            }
        }
        if card_ids.is_empty() {
            return Ok(None);
        }

        let output = if let Some(true) = self.randomise {
            for card in state.cards.values() {
                if card.get_controller_id(state) != *player_id {
                    continue;
                }

                if let Some(query) = card.card_query_override(state, self).await? {
                    let output = Box::pin(query.pick(player_id, state, use_preview)).await?;
                    if let Some(output) = output {
                        QueryCache::store_card_result(state.game_id, self.id, output);
                    }
                    return Ok(output);
                }
            }

            *card_ids
                .choose(&mut rand::rng())
                .expect("a card to be picked")
        } else {
            let prompt = self
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
                    self.source_card_id,
                )
                .await?
            } else {
                pick_card_source(player_id, &card_ids, state, &prompt, self.source_card_id).await?
            }
        };

        QueryCache::store_card_result(state.game_id, self.id, output);

        Ok(Some(output))
    }

    pub fn iter<'b>(&'b self, state: &'b State) -> impl Iterator<Item = &'b Box<dyn Card>> {
        state
            .cards
            .values()
            .filter(|c| self.matches(c.get_id(), state))
    }

    pub fn any(&self, state: &State) -> bool {
        state
            .cards
            .values()
            .any(|c| self.matches(c.get_id(), state))
    }

    pub fn first(&self, state: &State) -> Option<uuid::Uuid> {
        state
            .cards
            .values()
            .find(|c| self.matches(c.get_id(), state))
            .map(|c| *c.get_id())
    }

    pub fn all(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .values()
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

    pub fn with_source_card(self, card_id: uuid::Uuid) -> Self {
        Self {
            source_card_id: Some(card_id),
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
        self.adjacent_locations_to_any(zones)
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

    pub fn nearby_locations_to_card(mut self, card_id: &uuid::Uuid) -> Self {
        self.spatial_filters
            .push(SpatialFilter::NearbyLocationsToCard(*card_id));
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

    pub fn without_ability(self, ability: &Ability) -> Self {
        Self {
            without_abilities: Some(vec![ability.clone()]),
            ..self
        }
    }

    pub fn without_abilities(self, abilities: Vec<Ability>) -> Self {
        Self {
            without_abilities: Some(abilities),
            ..self
        }
    }

    pub fn with_ability(self, ability: &Ability) -> Self {
        Self {
            with_abilities: Some(vec![ability.clone()]),
            ..self
        }
    }

    pub fn with_abilities(self, abilities: Vec<Ability>) -> Self {
        Self {
            with_abilities: Some(abilities),
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

        if let Some(regions) = &self.regions
            && !regions.contains(card.get_region(state))
        {
            return false;
        }

        if self.spatial_filters.iter().any(|filter| {
            let zones = match filter {
                SpatialFilter::AdjacentLocations(zone) => zone.get_adjacent_locations(state),
                SpatialFilter::AdjacentLocationsToAny(zones) => zones
                    .iter()
                    .flat_map(|zone| zone.get_adjacent_locations(state))
                    .collect(),
                SpatialFilter::NearbyLocations(zone) => zone.get_nearby_locations(state),
                SpatialFilter::NearbyLocationsToCard(card_id) => state
                    .cards
                    .get(card_id)
                    .map(|card| card.get_zone().get_nearby_locations(state))
                    .unwrap_or_default(),
                SpatialFilter::AdjacentSites(zone) => zone.get_adjacent_sites(state),
                SpatialFilter::NearbySites(zone) => zone.get_nearby_sites(state),
                SpatialFilter::AdjacentVoids(zone) => zone.get_adjacent_voids(state),
                SpatialFilter::NearbyVoids(zone) => zone.get_nearby_voids(state),
            };
            !zones.iter().any(|zone| card.occupies_zone(state, zone))
        }) {
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
            && card.get_base().bearer != *carrier_id
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

        if let Some(abilities) = &self.with_abilities {
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
