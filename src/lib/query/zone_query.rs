use crate::{
    card::Region,
    game::{CardId, PlayerId},
    query::QueryCache,
    state::State,
    zone::{Location, Zone},
};

#[derive(Debug, Clone)]
pub(super) enum ZoneSpatialFilter {
    AdjacentLocations(Zone),
    NearbyLocations(Zone),
    ZoneOfCard(CardId),
    AffectedZonesOfCard(CardId),
}

#[derive(Debug, Clone)]
pub struct ZoneQuery {
    pub(super) id: uuid::Uuid,
    /// A fixed zone — resolves immediately without prompting the player.
    pub(super) zone: Option<Zone>,
    /// Explicit list of zones to pick from (or randomly select from when `random` is true).
    pub(super) options: Option<Vec<Zone>>,
    /// When true, a zone is chosen randomly from `options`.
    pub(super) random: bool,
    /// When true, the option pool is restricted to in-play site zones.
    pub(super) sites_only: bool,
    /// When true, the option pool is restricted to voids only.
    pub(super) voids_only: bool,
    /// Optionally filter `sites_only` results to zones controlled by this player.
    pub(super) controlled_by: Option<PlayerId>,
    pub(super) prompt: Option<String>,
    pub(super) source_card_id: Option<CardId>,
    pub(super) allow_modifiers: bool,
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
            voids_only: false,
            controlled_by: None,
            prompt: None,
            source_card_id: None,
            allow_modifiers: true,
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

    pub fn from_location(location: Location) -> Self {
        Self::from_zone(Zone::Location(location))
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

    pub fn zone_of_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::ZoneOfCard(*card_id));
        self
    }

    pub fn affected_zones_of_card(mut self, card_id: &CardId) -> Self {
        self.spatial_filters
            .push(ZoneSpatialFilter::AffectedZonesOfCard(*card_id));
        self
    }

    /// Player picks from in-play site zones, optionally filtered by controller.
    pub fn any_void() -> Self {
        Self {
            voids_only: true,
            ..Self::default()
        }
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

    /// A zone is chosen randomly from `options`.
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

    pub fn with_source_card(self, card_id: CardId) -> Self {
        Self {
            source_card_id: Some(card_id),
            ..self
        }
    }

    pub fn source_card_id(&self) -> Option<CardId> {
        self.source_card_id
    }

    pub fn without_modifiers(self) -> Self {
        Self {
            allow_modifiers: false,
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
                        .map(|aura| {
                            aura.get_affected_zones(state)
                                .iter()
                                .map(|l| l.clone().into())
                                .collect()
                        })
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
        } else if self.voids_only {
            let all_voids = Location::all_in_region(Region::Surface)
                .into_iter()
                .filter(|l| l.get_site(state).is_none())
                .map(|l| l.into())
                .collect();
            (all_voids, self.spatial_filters.as_slice())
        } else {
            (
                Location::all_in_region(Region::Surface)
                    .into_iter()
                    .map(|l| l.into())
                    .collect(),
                self.spatial_filters.as_slice(),
            )
        };

        for filter in filters_to_apply {
            let allowed_zones = filter_zones(filter);
            zones.retain(|zone| allowed_zones.contains(zone));
        }

        zones
    }

    pub fn matches(&self, state: &State, zone: &Zone) -> bool {
        self.options(state).contains(zone)
    }

    /// Resolves the query, prompting the player if needed. Caches the result.
    pub async fn pick(&self, player_id: &PlayerId, state: &State) -> anyhow::Result<Zone> {
        QueryCache::resolve_zone(self, player_id, state).await
    }
}

#[derive(Debug, Clone)]
pub struct LocationQuery {
    zone_query: ZoneQuery,
}

impl From<Location> for LocationQuery {
    fn from(location: Location) -> Self {
        Self::from_location(location)
    }
}

impl From<Zone> for LocationQuery {
    fn from(zone: Zone) -> Self {
        Self::from_zone(zone)
    }
}

impl From<&Zone> for LocationQuery {
    fn from(zone: &Zone) -> Self {
        Self::from_zone(zone.clone())
    }
}

impl Default for LocationQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationQuery {
    pub fn new() -> Self {
        Self {
            zone_query: ZoneQuery::new(),
        }
    }

    pub fn from_location(location: Location) -> Self {
        Self {
            zone_query: ZoneQuery::from_zone(Zone::Location(location)),
        }
    }

    pub fn from_zone(zone: impl Into<Zone>) -> Self {
        let zone = zone.into();
        let Zone::Location(location) = zone else {
            panic!("LocationQuery::from_zone requires an in-play location");
        };
        Self::from_location(location)
    }

    pub fn from_options(options: Vec<Zone>, prompt: Option<String>) -> Self {
        Self {
            zone_query: ZoneQuery::from_options(options, prompt),
        }
    }

    pub fn from_locations(options: Vec<Location>, prompt: Option<String>) -> Self {
        Self {
            zone_query: ZoneQuery::from_options(
                options.into_iter().map(Zone::from).collect(),
                prompt,
            ),
        }
    }

    pub fn random(options: Vec<Location>) -> Self {
        Self {
            zone_query: ZoneQuery::random(options.into_iter().map(|l| l.into()).collect()),
        }
    }

    pub fn any_void() -> Self {
        Self {
            zone_query: ZoneQuery::any_void(),
        }
    }

    pub fn any_site(controlled_by: Option<PlayerId>, prompt: Option<String>) -> Self {
        Self {
            zone_query: ZoneQuery::any_site(controlled_by, prompt),
        }
    }

    pub fn zone_of_card(mut self, card_id: &CardId) -> Self {
        self.zone_query = self.zone_query.zone_of_card(card_id);
        self
    }

    pub fn affected_zones_of_card(mut self, card_id: &CardId) -> Self {
        self.zone_query = self.zone_query.affected_zones_of_card(card_id);
        self
    }

    pub fn options(&self, state: &State) -> Vec<Location> {
        self.zone_query
            .options(state)
            .into_iter()
            .filter_map(|zone| match zone {
                Zone::Location(location) => Some(location),
                _ => None,
            })
            .collect()
    }

    pub async fn pick(&self, player_id: &PlayerId, state: &State) -> anyhow::Result<Location> {
        match self.zone_query.pick(player_id, state).await? {
            Zone::Location(location) => Ok(location),
            zone => Err(anyhow::anyhow!("expected location, got {zone}")),
        }
    }
}
