use crate::{game::PlayerId, query::QueryCache, state::State, zone::Zone};

#[derive(Debug, Clone)]
pub(super) enum ZoneSpatialFilter {
    AdjacentLocations(Zone),
    NearbyLocations(Zone),
    ZoneOfCard(uuid::Uuid),
    AffectedZonesOfCard(uuid::Uuid),
}

#[derive(Debug, Clone)]
pub struct ZoneQuery {
    pub(super) id: uuid::Uuid,
    /// A fixed zone — resolves immediately without prompting the player.
    pub(super) zone: Option<Zone>,
    /// Explicit list of zones to pick from (or randomly select from when `random` is true).
    pub(super) options: Option<Vec<Zone>>,
    /// When true, a zone is chosen randomly from `options` (subject to `zone_query_override`).
    pub(super) random: bool,
    /// When true, the option pool is restricted to in-play site zones.
    pub(super) sites_only: bool,
    /// Optionally filter `sites_only` results to zones controlled by this player.
    pub(super) controlled_by: Option<PlayerId>,
    pub(super) prompt: Option<String>,
    pub(super) source_card_id: Option<uuid::Uuid>,
    pub(super) spatial_filters: Vec<ZoneSpatialFilter>,
}

impl Default for ZoneQuery {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            zone: None,
            options: None,
            random: false,
            sites_only: false,
            controlled_by: None,
            prompt: None,
            source_card_id: None,
            spatial_filters: vec![],
        }
    }
}

impl From<Zone> for ZoneQuery {
    fn from(zone: Zone) -> Self {
        ZoneQuery::from_zone(zone)
    }
}

impl From<&Zone> for ZoneQuery {
    fn from(zone: &Zone) -> Self {
        ZoneQuery::from_zone(zone.clone())
    }
}

impl ZoneQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_randomised(&self) -> bool {
        self.random
    }

    /// Resolves to a specific zone without prompting.
    pub fn from_zone(zone: Zone) -> Self {
        Self {
            zone: Some(zone),
            ..Self::default()
        }
    }

    pub fn adjacent_to(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::AdjacentLocations(zone.clone()));
        self
    }

    pub fn near(mut self, zone: &Zone) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::NearbyLocations(zone.clone()));
        self
    }

    pub fn zone_of_card(mut self, card_id: &uuid::Uuid) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::ZoneOfCard(*card_id));
        self
    }

    pub fn affected_zones_of_card(mut self, card_id: &uuid::Uuid) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::AffectedZonesOfCard(*card_id));
        self
    }

    /// Player picks from in-play site zones, optionally filtered by controller.
    pub fn any_site(controlled_by: Option<PlayerId>, prompt: Option<String>) -> Self {
        Self {
            sites_only: true,
            controlled_by,
            prompt,
            ..Self::default()
        }
    }

    /// A zone is chosen randomly from `options` (cards may override via `zone_query_override`).
    pub fn random(options: Vec<Zone>) -> Self {
        Self {
            options: Some(options),
            random: true,
            ..Self::default()
        }
    }

    /// Player picks from the given list of zones.
    pub fn from_options(options: Vec<Zone>, prompt: Option<String>) -> Self {
        Self {
            options: Some(options),
            prompt,
            ..Self::default()
        }
    }

    pub fn with_prompt(self, prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            ..self
        }
    }

    pub fn with_source_card(self, card_id: uuid::Uuid) -> Self {
        Self {
            source_card_id: Some(card_id),
            ..self
        }
    }

    pub(super) fn prompt(&self) -> &str {
        self.prompt.as_deref().unwrap_or(if self.sites_only {
            "Pick a site zone"
        } else {
            "Pick a zone"
        })
    }

    /// Returns the set of candidate zones for this query given current game state.
    pub fn options(&self, state: &State) -> Vec<Zone> {
        if let Some(zone) = &self.zone {
            return vec![zone.clone()];
        }

        let filter_zones = |filter: &ZoneSpatialFilter| match filter {
            ZoneSpatialFilter::AdjacentLocations(zone) => zone.get_adjacent_locations(state),
            ZoneSpatialFilter::NearbyLocations(zone) => zone.get_nearby_locations(state),
            ZoneSpatialFilter::ZoneOfCard(card_id) => state
                .cards
                .get(card_id)
                .map(|card| vec![card.get_zone().clone()])
                .unwrap_or_default(),
            ZoneSpatialFilter::AffectedZonesOfCard(card_id) => state
                .cards
                .get(card_id)
                .map(|card| {
                    card.get_aura()
                        .map(|aura| aura.get_affected_zones(state))
                        .unwrap_or_else(|| vec![card.get_zone().clone()])
                })
                .unwrap_or_default(),
        };

        let (mut zones, filters_to_apply) = if let Some(opts) = &self.options {
            (opts.clone(), self.spatial_filters.as_slice())
        } else if let Some(first_filter) = self.spatial_filters.first()
            && !self.sites_only
        {
            (filter_zones(first_filter), &self.spatial_filters[1..])
        } else if self.sites_only {
            let mut sites: Vec<Zone> = state
                .cards
                .values()
                .filter(|c| c.is_site())
                .filter(|c| c.get_zone().is_in_play())
                .filter(|c| {
                    self.controlled_by
                        .as_ref()
                        .is_none_or(|p| c.get_controller_id(state) == *p)
                })
                .map(|c| c.get_zone().clone())
                .collect();
            sites.dedup();
            (sites, self.spatial_filters.as_slice())
        } else {
            (Zone::all_realm(), self.spatial_filters.as_slice())
        };

        for filter in filters_to_apply {
            let allowed_zones = filter_zones(filter);
            zones.retain(|zone| allowed_zones.contains(zone));
        }

        zones
    }

    /// Resolves the query, prompting the player if needed. Caches the result.
    pub async fn pick(&self, player_id: &PlayerId, state: &State) -> anyhow::Result<Zone> {
        QueryCache::resolve_zone(self, player_id, state).await
    }
}
