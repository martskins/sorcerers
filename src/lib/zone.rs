use crate::{
    card::{Ability, CardType, Region, Rubble, Site},
    game::{
        CardId, Direction, PlayerId, are_adjacent, are_nearby, get_adjacent_zones, get_nearby_zones,
    },
    query::CardQuery,
    state::{OngoingEffect, State},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub enum Location {
    Square(u8, Region),
    Intersection(Vec<u8>, Region),
}

impl From<Location> for Zone {
    fn from(value: Location) -> Self {
        Zone::Location(value)
    }
}

impl From<&Location> for Zone {
    fn from(value: &Location) -> Self {
        Zone::Location(value.clone())
    }
}

impl From<&Zone> for Zone {
    fn from(value: &Zone) -> Self {
        value.clone()
    }
}

impl Location {
    pub fn region(&self) -> &Region {
        match self {
            Location::Square(_, region) | Location::Intersection(_, region) => region,
        }
    }

    pub fn squares(&self) -> Vec<u8> {
        match self {
            Location::Square(square, _) => vec![*square],
            Location::Intersection(squares, _) => squares.clone(),
        }
    }

    pub fn square(&self) -> Option<u8> {
        match self {
            Location::Square(square, _) => Some(*square),
            Location::Intersection(_, _) => None,
        }
    }

    pub fn get_square(&self) -> Option<u8> {
        self.square()
    }

    pub fn all_in_region(region: Region) -> Vec<Location> {
        (1..=20)
            .map(|sq| Location::Square(sq, region.clone()))
            .collect()
    }

    pub fn with_region(&self, region: Region) -> Self {
        match self {
            Location::Square(square, _) => Location::Square(*square, region),
            Location::Intersection(squares, _) => Location::Intersection(squares.clone(), region),
        }
    }

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        if matches!(self, Location::Square(_, _)) {
            return self.get_site_at_square(state);
        }

        CardQuery::new()
            .sites()
            .in_location(self.clone())
            .first(state)
            .and_then(|site_id| state.get_card(&site_id).get_site())
    }

    pub fn get_site_at_square<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        let square = self.square()?;
        state
            .cards
            .values()
            .find(|card| {
                card.is_site()
                    && card.get_zone().is_in_play()
                    && card.get_zone().get_square() == Some(square)
            })
            .and_then(|card| card.get_site())
    }

    pub fn get_adjacent(&self) -> Vec<Self> {
        match self {
            Location::Square(square, region) => {
                let mut adjacent = match square % 5 {
                    0 => vec![
                        Location::Square(square.saturating_add(5), region.clone()),
                        Location::Square(square.saturating_sub(5), region.clone()),
                        Location::Square(square.saturating_sub(1), region.clone()),
                        Location::Square(*square, region.clone()),
                    ],
                    1 => vec![
                        Location::Square(square.saturating_add(5), region.clone()),
                        Location::Square(square.saturating_sub(5), region.clone()),
                        Location::Square(square.saturating_add(1), region.clone()),
                        Location::Square(*square, region.clone()),
                    ],
                    _ => vec![
                        Location::Square(square.saturating_add(5), region.clone()),
                        Location::Square(square.saturating_sub(5), region.clone()),
                        Location::Square(square.saturating_add(1), region.clone()),
                        Location::Square(square.saturating_sub(1), region.clone()),
                        Location::Square(*square, region.clone()),
                    ],
                };
                adjacent.retain(|s| s.square().unwrap_or(0) <= 20);
                adjacent.retain(|s| s.square().unwrap_or(0) > 0);
                adjacent
            }
            Location::Intersection(locs, region) => {
                let mut locs = locs.clone();
                locs.sort();
                let mut intersections = vec![
                    Location::Intersection(
                        locs.iter().map(|l| l.saturating_add(5)).collect(),
                        region.clone(),
                    ),
                    Location::Intersection(
                        locs.iter().map(|l| l.saturating_add(1)).collect(),
                        region.clone(),
                    ),
                ];

                if locs[0] > 1 {
                    intersections.push(Location::Intersection(
                        locs.iter().map(|l| l.saturating_sub(1)).collect(),
                        region.clone(),
                    ));
                }

                if locs[0] > 5 {
                    intersections.push(Location::Intersection(
                        locs.iter().map(|l| l.saturating_sub(5)).collect(),
                        region.clone(),
                    ));
                }

                intersections
            }
        }
    }

    pub fn get_nearby(&self) -> Vec<Self> {
        get_nearby_zones(&Zone::from(self))
            .into_iter()
            .filter_map(Zone::into_location)
            .collect()
    }

    pub fn get_nearby_locations(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter(|location| Zone::from(location).is_location(state))
            .collect()
    }

    pub fn get_adjacent_locations(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter(|location| Zone::from(location).is_location(state))
            .collect()
    }

    pub fn get_nearby_sites(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter(|location| location.get_site(state).is_some())
            .collect()
    }

    pub fn get_adjacent_sites(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter(|location| location.get_site(state).is_some())
            .collect()
    }

    pub fn get_nearby_voids(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let void = Location::Square(square, Region::Void);
                Zone::from(&void).is_location(state).then_some(void)
            })
            .collect()
    }

    pub fn get_adjacent_voids(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let void = Location::Square(square, Region::Void);
                Zone::from(&void).is_location(state).then_some(void)
            })
            .collect()
    }

    pub fn steps_in_direction(&self, direction: &Direction, steps: u8) -> Option<Self> {
        let mut current_zone = self.clone();
        for _ in 0..steps {
            match current_zone.step_in_direction(direction) {
                Some(z) => current_zone = z,
                None => return None,
            }
        }
        Some(current_zone)
    }

    pub fn zone_in_direction(&self, direction: &Direction, steps: u8) -> Option<Zone> {
        self.steps_in_direction(direction, steps).map(Zone::from)
    }

    pub fn step_in_direction(&self, direction: &Direction) -> Option<Self> {
        match self {
            Location::Square(square, region) => {
                let zone = match direction {
                    Direction::Up => Location::Square(square.saturating_add(5), region.clone()),
                    Direction::Down => Location::Square(square.saturating_sub(5), region.clone()),
                    Direction::Left => Location::Square(square.saturating_sub(1), region.clone()),
                    Direction::Right => Location::Square(square.saturating_add(1), region.clone()),
                    Direction::TopLeft => {
                        Location::Square(square.saturating_add(4), region.clone())
                    }
                    Direction::TopRight => {
                        Location::Square(square.saturating_add(6), region.clone())
                    }
                    Direction::BottomLeft => {
                        Location::Square(square.saturating_sub(6), region.clone())
                    }
                    Direction::BottomRight => {
                        Location::Square(square.saturating_sub(4), region.clone())
                    }
                };

                match direction {
                    Direction::Up | Direction::Down => {
                        if zone.square() > Some(20) || zone.square() < Some(1) {
                            return None;
                        }

                        Some(zone)
                    }
                    _ => Some(zone),
                }
            }
            Location::Intersection(locs, region) => {
                let new_squares: Vec<u8> = locs
                    .iter()
                    .filter_map(|sq| {
                        let realm_zone = Zone::Location(Location::Square(*sq, region.clone()));
                        realm_zone.zone_in_direction(direction, 1)?.get_square()
                    })
                    .collect();

                for intersection in Zone::all_intersections() {
                    if let Zone::Location(Location::Intersection(ilocs, _)) = &intersection
                        && ilocs == &new_squares
                    {
                        return Some(Location::Intersection(new_squares, region.clone()));
                    }
                }

                None
            }
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Square(square, region) => write!(f, "{} ({})", square, region),
            Location::Intersection(squares, region) => write!(
                f,
                "Intersection of ({}) ({})",
                squares
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
                region
            ),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub enum Zone {
    #[default]
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Location(Location),
    Cemetery,
    Banish,
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::None => write!(f, "None"),
            Zone::Hand => write!(f, "Hand"),
            Zone::Spellbook => write!(f, "Spellbook"),
            Zone::Atlasbook => write!(f, "Atlasbook"),
            Zone::Location(Location::Square(sq, region)) => write!(f, "{} ({})", sq, region),
            Zone::Cemetery => write!(f, "Cemetery"),
            Zone::Banish => write!(f, "Banish"),
            Zone::Location(Location::Intersection(locs, region)) => write!(
                f,
                "Intersection of ({}) ({})",
                locs.iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
                region
            ),
        }
    }
}

impl Zone {
    pub fn location(&self) -> Option<&Location> {
        match self {
            Zone::Location(location) => Some(location),
            _ => None,
        }
    }

    pub fn into_location(self) -> Option<Location> {
        match self {
            Zone::Location(location) => Some(location),
            _ => None,
        }
    }

    pub fn squares(&self) -> Vec<u8> {
        match self {
            Zone::Location(Location::Square(square, _)) => vec![*square],
            Zone::Location(Location::Intersection(squares, _)) => squares.clone(),
            _ => vec![],
        }
    }

    pub fn occupied_regions(&self, state: &State) -> Vec<Region> {
        match self {
            Zone::Location(Location::Square(square, region)) => {
                vec![Self::occupied_square_region(*square, region, state)]
            }
            Zone::Location(Location::Intersection(squares, region)) => {
                let mut regions = squares
                    .iter()
                    .map(|square| Self::occupied_square_region(*square, region, state))
                    .collect::<Vec<_>>();
                regions.sort();
                regions.dedup();
                regions
            }
            _ => vec![],
        }
    }

    fn occupied_square_region(square: u8, region: &Region, state: &State) -> Region {
        let zone = Zone::Location(Location::Square(square, region.clone()));
        let Some(site) = zone.get_site_at_square(state) else {
            return Region::Void;
        };

        match region {
            Region::Void | Region::Surface => Region::Surface,
            Region::Underground | Region::Underwater => {
                if site.is_water_site(state).unwrap_or_default() {
                    Region::Underwater
                } else {
                    Region::Underground
                }
            }
        }
    }

    pub fn is_in_play(&self) -> bool {
        matches!(
            self,
            Zone::Location(Location::Square(_, _)) | Zone::Location(Location::Intersection(_, _))
        )
    }

    pub fn can_be_entered_by(&self, state: &State, card_id: &CardId) -> anyhow::Result<bool> {
        let mut can_enter = true;
        for ce in state.active_continuous_effects() {
            match ce {
                OngoingEffect::MakeZoneUnvisitable {
                    location,
                    affected_cards,
                } if location == self.location().unwrap()
                    && affected_cards.matches(card_id, state) =>
                {
                    can_enter = false;
                    break;
                }
                _ => {}
            }
        }

        Ok(can_enter)
    }

    pub fn is_valid_play_zone_for(
        &self,
        state: &State,
        card_id: &CardId,
        player_id: &PlayerId,
    ) -> anyhow::Result<bool> {
        if !self.is_in_play() {
            return Ok(false);
        }

        let should_override = state
            .active_continuous_effects()
            .into_iter()
            .filter(|e| match e {
                OngoingEffect::OverrideValidPlayZone {
                    affected_locations,
                    affected_cards,
                    ..
                } => self.location().is_some_and(|location| {
                    affected_locations.options(state).contains(location)
                        && affected_cards.matches(card_id, state)
                }),
                _ => false,
            })
            .count()
            > 0;
        if should_override {
            return Ok(true);
        }

        match self {
            Zone::Location(Location::Square(_, _)) => {
                let card = state.get_card(card_id);
                // Auras should be played on intersections unless otherwise stated.
                if card.get_card_type() == CardType::Aura {
                    return Ok(false);
                }

                let site_in_zone = self.get_site(state);
                if let Some(site) = site_in_zone {
                    return site.is_valid_play_site_for(state, card_id, player_id);
                }

                // If there's no site in the zone, only cards with Voidwalk can be played there.
                match card.get_card_type() {
                    CardType::Site => {
                        let has_played_site = !CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(player_id)
                            .all(state)
                            .is_empty();
                        if !has_played_site {
                            let avatar_id = state.get_player_avatar_id(player_id)?;
                            let avatar = state.get_card(&avatar_id);
                            return Ok(avatar.get_zone() == self);
                        }

                        let empty_adjacent_zones: Vec<Zone> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(player_id)
                            .not_named(Rubble::NAME.to_string())
                            .all(state)
                            .into_iter()
                            .map(|cid| state.get_card(&cid).get_zone())
                            .flat_map(|z| z.get_adjacent())
                            .filter(|z| z.get_site(state).is_none())
                            .collect();

                        Ok(empty_adjacent_zones.contains(self))
                    }
                    _ => Ok(card.has_ability(state, &Ability::Voidwalk)),
                }
            }
            Zone::Location(Location::Intersection(sqs, region)) => {
                let card = state.get_card(card_id);
                match card.get_card_type() {
                    CardType::Minion => {
                        if !card.is_oversized(state) {
                            return Ok(false);
                        }

                        let site_squares: Vec<u8> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(player_id)
                            .all(state)
                            .into_iter()
                            .filter_map(|cid| state.get_card(&cid).get_zone().get_square())
                            .collect();
                        Ok(sqs.iter().any(|sq| site_squares.contains(sq)))
                    }
                    CardType::Aura => Ok(matches!(region, Region::Surface | Region::Void)),
                    _ => Ok(false),
                }
            }
            _ => Ok(false),
        }
    }

    pub fn steps_to_zone(&self, other: &Zone) -> Option<u8> {
        self.min_steps_to_zone(other)
    }

    pub fn min_steps_to_zone(&self, other: &Zone) -> Option<u8> {
        let mut visited = Vec::new();
        let mut to_visit = vec![(self.clone(), 0)];

        while let Some((current_zone, current_step)) = to_visit.pop() {
            if &current_zone == other {
                return Some(current_step);
            }

            if !visited.contains(&current_zone) {
                visited.push(current_zone.clone());

                for adjacent in current_zone.get_adjacent() {
                    to_visit.push((adjacent, current_step + 1));
                }
            }
        }

        None
    }

    pub fn all_intersections() -> Vec<Zone> {
        vec![
            Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface)),
            Zone::Location(Location::Intersection(vec![2, 3, 7, 8], Region::Surface)),
            Zone::Location(Location::Intersection(vec![3, 4, 8, 9], Region::Surface)),
            Zone::Location(Location::Intersection(vec![4, 5, 9, 10], Region::Surface)),
            Zone::Location(Location::Intersection(vec![6, 7, 11, 12], Region::Surface)),
            Zone::Location(Location::Intersection(vec![7, 8, 12, 13], Region::Surface)),
            Zone::Location(Location::Intersection(vec![8, 9, 13, 14], Region::Surface)),
            Zone::Location(Location::Intersection(vec![9, 10, 14, 15], Region::Surface)),
            Zone::Location(Location::Intersection(
                vec![11, 12, 16, 17],
                Region::Surface,
            )),
            Zone::Location(Location::Intersection(
                vec![12, 13, 17, 18],
                Region::Surface,
            )),
            Zone::Location(Location::Intersection(
                vec![13, 14, 18, 19],
                Region::Surface,
            )),
            Zone::Location(Location::Intersection(
                vec![14, 15, 19, 20],
                Region::Surface,
            )),
        ]
    }

    pub fn all_in_surface() -> Vec<Zone> {
        (1..=20)
            .map(|sq| Zone::Location(Location::Square(sq, Region::Surface)))
            .collect()
    }

    pub fn all_in_region(region: Region) -> Vec<Zone> {
        (1..=20)
            .map(|sq| Zone::Location(Location::Square(sq, region.clone())))
            .collect()
    }

    pub fn all_realm() -> Vec<Zone> {
        (1..=20)
            .map(|sq| Zone::Location(Location::Square(sq, Region::Surface)))
            .collect()
    }

    pub fn all_board() -> Vec<Zone> {
        let mut zones = Self::all_realm();
        zones.extend(Self::all_intersections());
        zones
    }

    pub fn get_square(&self) -> Option<u8> {
        match self {
            Zone::Location(Location::Square(sq, _)) => Some(*sq),
            _ => None,
        }
    }

    pub fn with_region(&self, region: Region) -> Zone {
        match self {
            Zone::Location(Location::Square(square, _)) => {
                Zone::Location(Location::Square(*square, region))
            }
            Zone::Location(Location::Intersection(squares, _)) => {
                Zone::Location(Location::Intersection(squares.clone(), region))
            }
            zone => zone.clone(),
        }
    }

    pub fn is_nearby(&self, other: &Zone) -> bool {
        are_nearby(self, other)
    }

    pub fn is_adjacent(&self, other: &Zone) -> bool {
        are_adjacent(self, other)
    }

    pub fn get_nearby(&self) -> Vec<Zone> {
        get_nearby_zones(self)
    }

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        if matches!(self, Zone::Location(Location::Square(_, _))) {
            return self.get_site_at_square(state);
        }

        CardQuery::new()
            .sites()
            .in_zone(self)
            .first(state)
            .and_then(|site_id| state.get_card(&site_id).get_site())
    }

    pub fn get_site_at_square<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        let square = self.get_square()?;
        state
            .cards
            .values()
            .find(|card| {
                card.is_site()
                    && card.get_zone().is_in_play()
                    && card.get_zone().get_square() == Some(square)
            })
            .and_then(|card| card.get_site())
    }

    pub fn get_nearby_locations(&self, state: &State) -> Vec<Zone> {
        self.get_nearby()
            .into_iter()
            .filter(|zone| zone.is_location(state))
            .collect()
    }

    pub fn get_adjacent_locations(&self, state: &State) -> Vec<Zone> {
        self.get_adjacent()
            .into_iter()
            .filter(|zone| zone.is_location(state))
            .collect()
    }

    pub fn get_nearby_sites(&self, state: &State) -> Vec<Zone> {
        self.cross_region_nearby_zones(true)
            .into_iter()
            .filter(|zone| zone.get_site(state).is_some())
            .collect()
    }

    pub fn get_adjacent_sites(&self, state: &State) -> Vec<Zone> {
        self.cross_region_nearby_zones(false)
            .into_iter()
            .filter(|zone| zone.get_site(state).is_some())
            .collect()
    }

    pub fn get_nearby_voids(&self, state: &State) -> Vec<Zone> {
        self.cross_region_nearby_zones(true)
            .into_iter()
            .filter_map(|zone| {
                let square = zone.get_square()?;
                let void = Zone::Location(Location::Square(square, Region::Void));
                void.is_location(state).then_some(void)
            })
            .collect()
    }

    pub fn get_adjacent_voids(&self, state: &State) -> Vec<Zone> {
        self.cross_region_nearby_zones(false)
            .into_iter()
            .filter_map(|zone| {
                let square = zone.get_square()?;
                let void = Zone::Location(Location::Square(square, Region::Void));
                void.is_location(state).then_some(void)
            })
            .collect()
    }

    fn cross_region_nearby_zones(&self, include_diagonals: bool) -> Vec<Zone> {
        let Some(square) = self.get_square() else {
            return vec![];
        };
        let zone = Zone::Location(Location::Square(square, Region::Surface));
        if include_diagonals {
            zone.get_nearby()
        } else {
            zone.get_adjacent()
        }
    }

    fn is_location(&self, state: &State) -> bool {
        match self {
            Zone::Location(Location::Square(_, Region::Surface)) => {
                self.get_site_at_square(state).is_some()
            }
            Zone::Location(Location::Square(_, Region::Void)) => {
                self.get_site_at_square(state).is_none()
            }
            Zone::Location(Location::Square(_, Region::Underground)) => self
                .get_site_at_square(state)
                .is_some_and(|site| site.is_land_site(state).unwrap_or_default()),
            Zone::Location(Location::Square(_, Region::Underwater)) => self
                .get_site_at_square(state)
                .is_some_and(|site| site.is_water_site(state).unwrap_or_default()),
            Zone::Location(Location::Intersection(squares, region)) => {
                squares.iter().all(|square| {
                    Zone::Location(Location::Square(*square, region.clone())).is_location(state)
                })
            }
            _ => false,
        }
    }

    pub fn zone_in_direction(&self, direction: &Direction, steps: u8) -> Option<Self> {
        let mut current_zone = self.clone();
        for _ in 0..steps {
            match current_zone.step_in_direction(direction) {
                Some(z) => current_zone = z,
                None => return None,
            }
        }
        Some(current_zone)
    }

    fn step_in_direction(&self, direction: &Direction) -> Option<Self> {
        match self {
            Zone::Location(Location::Square(square, region)) => {
                let zone = match direction {
                    Direction::Up => {
                        Zone::Location(Location::Square(square.saturating_add(5), region.clone()))
                    }
                    Direction::Down => {
                        Zone::Location(Location::Square(square.saturating_sub(5), region.clone()))
                    }
                    Direction::Left => {
                        Zone::Location(Location::Square(square.saturating_sub(1), region.clone()))
                    }
                    Direction::Right => {
                        Zone::Location(Location::Square(square.saturating_add(1), region.clone()))
                    }
                    Direction::TopLeft => {
                        Zone::Location(Location::Square(square.saturating_add(4), region.clone()))
                    }
                    Direction::TopRight => {
                        Zone::Location(Location::Square(square.saturating_add(6), region.clone()))
                    }
                    Direction::BottomLeft => {
                        Zone::Location(Location::Square(square.saturating_sub(6), region.clone()))
                    }
                    Direction::BottomRight => {
                        Zone::Location(Location::Square(square.saturating_sub(4), region.clone()))
                    }
                };

                match direction {
                    Direction::Up | Direction::Down => {
                        if zone.get_square() > Some(20) || zone.get_square() < Some(1) {
                            return None;
                        }

                        Some(zone)
                    }
                    _ => Some(zone),
                }
            }
            Zone::Location(Location::Intersection(locs, region)) => {
                let new_squares: Vec<u8> = locs
                    .iter()
                    .filter_map(|sq| {
                        let realm_zone = Zone::Location(Location::Square(*sq, region.clone()));
                        realm_zone.zone_in_direction(direction, 1)?.get_square()
                    })
                    .collect();

                for intersection in Zone::all_intersections() {
                    if let Zone::Location(Location::Intersection(ilocs, _)) = &intersection
                        && ilocs == &new_squares
                    {
                        return Some(Zone::Location(Location::Intersection(
                            new_squares,
                            region.clone(),
                        )));
                    }
                }

                None
            }
            _ => None,
        }
    }

    pub fn get_adjacent(&self) -> Vec<Zone> {
        get_adjacent_zones(self)
    }
}
