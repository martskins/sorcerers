use crate::{
    card::{Ability, CardType, Region, Rubble, Site},
    game::{CardId, Direction, PlayerId},
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

    pub fn can_be_entered_by(&self, state: &State, card_id: &CardId) -> anyhow::Result<bool> {
        let mut can_enter = true;
        for ce in state.active_continuous_effects() {
            match ce {
                OngoingEffect::MakeZoneUnvisitable {
                    location,
                    affected_cards,
                } if location == self && affected_cards.matches(card_id, state) => {
                    can_enter = false;
                    break;
                }
                _ => {}
            }
        }

        Ok(can_enter)
    }

    pub fn square_after_steps_in_direction(
        square: u8,
        direction: &Direction,
        steps: u8,
        state: &State,
        source_card_id: Option<&CardId>,
    ) -> Option<u8> {
        let wraps_top_bottom = Self::connects_top_bottom_edges(state, source_card_id);
        let wraps_left_right = Self::connects_left_right_edges(state, source_card_id);
        let mut current_square = square;
        for _ in 0..steps {
            current_square = Self::next_square_in_direction(
                current_square,
                direction,
                wraps_top_bottom,
                wraps_left_right,
            )?;
        }

        Some(current_square)
    }

    pub fn square_in_direction(
        &self,
        direction: &Direction,
        steps: u8,
        state: &State,
        source_card_id: Option<&CardId>,
    ) -> Option<u8> {
        Self::square_after_steps_in_direction(
            self.square()?,
            direction,
            steps,
            state,
            source_card_id,
        )
    }

    fn next_square_in_direction(
        square: u8,
        direction: &Direction,
        wraps_top_bottom: bool,
        wraps_left_right: bool,
    ) -> Option<u8> {
        if !(1..=20).contains(&square) {
            return None;
        }

        let row = (square - 1) / 5;
        let col = (square - 1) % 5;
        let (row_delta, col_delta) = match direction {
            Direction::Up => (1, 0),
            Direction::Down => (-1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1),
            Direction::TopLeft => (1, -1),
            Direction::TopRight => (1, 1),
            Direction::BottomLeft => (-1, -1),
            Direction::BottomRight => (-1, 1),
        };

        let mut next_row = row as i8 + row_delta;
        if !(0..=3).contains(&next_row) {
            if !wraps_top_bottom {
                return None;
            }
            next_row = next_row.rem_euclid(4);
        }

        let mut next_col = col as i8 + col_delta;
        if !(0..=4).contains(&next_col) {
            if !wraps_left_right {
                return None;
            }
            next_col = next_col.rem_euclid(5);
        }

        Some((next_row as u8 * 5) + next_col as u8 + 1)
    }

    fn connects_top_bottom_edges(state: &State, source_card_id: Option<&CardId>) -> bool {
        state.active_continuous_effects().into_iter().any(|ce| {
            matches!(
                ce,
                OngoingEffect::ConnectTopBottomEdges { affected_cards }
                    if source_card_id.is_some_and(|card_id| affected_cards.matches(card_id, state))
            )
        })
    }

    fn connects_left_right_edges(state: &State, source_card_id: Option<&CardId>) -> bool {
        state.active_continuous_effects().into_iter().any(|ce| {
            matches!(
                ce,
                OngoingEffect::ConnectLeftRightEdges { affected_cards }
                    if source_card_id.is_some_and(|card_id| affected_cards.matches(card_id, state))
            )
        })
    }

    pub fn all() -> Vec<Location> {
        let mut all = Location::all_in_region(Region::Surface);
        all.extend(Location::all_in_region(Region::Underground));
        all.extend(Location::all_in_region(Region::Underwater));
        all.extend(Location::all_in_region(Region::Void));
        all
    }

    pub fn all_in_region(region: Region) -> Vec<Location> {
        (1..=20)
            .map(|sq| Location::Square(sq, region.clone()))
            .collect()
    }

    pub fn all_intersections() -> Vec<Location> {
        vec![
            Location::Intersection(vec![1, 2, 6, 7], Region::Surface),
            Location::Intersection(vec![2, 3, 7, 8], Region::Surface),
            Location::Intersection(vec![3, 4, 8, 9], Region::Surface),
            Location::Intersection(vec![4, 5, 9, 10], Region::Surface),
            Location::Intersection(vec![6, 7, 11, 12], Region::Surface),
            Location::Intersection(vec![7, 8, 12, 13], Region::Surface),
            Location::Intersection(vec![8, 9, 13, 14], Region::Surface),
            Location::Intersection(vec![9, 10, 14, 15], Region::Surface),
            Location::Intersection(vec![11, 12, 16, 17], Region::Surface),
            Location::Intersection(vec![12, 13, 17, 18], Region::Surface),
            Location::Intersection(vec![13, 14, 18, 19], Region::Surface),
            Location::Intersection(vec![14, 15, 19, 20], Region::Surface),
        ]
    }

    pub fn with_region(&self, region: Region) -> Self {
        match self {
            Location::Square(square, _) => Location::Square(*square, region),
            Location::Intersection(squares, _) => Location::Intersection(squares.clone(), region),
        }
    }

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        match self {
            Location::Square(_, _) => self.get_site_at_square(state),
            _ => None,
        }
    }

    pub fn get_site_at_square<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        let square = self.square()?;
        dbg!(
            CardQuery::new()
                .sites()
                .in_location(Location::Square(square, Region::Surface))
                .first(state)
                .and_then(|site_id| state.get_card(&site_id).get_site())
        )
    }

    pub fn is_location(&self, state: &State) -> bool {
        match self {
            Location::Square(_, Region::Surface) => self.get_site_at_square(state).is_some(),
            Location::Square(_, Region::Void) => self.get_site_at_square(state).is_none(),
            Location::Square(_, Region::Underground) => self
                .get_site_at_square(state)
                .is_some_and(|site| site.is_land_site(state).unwrap_or_default()),
            Location::Square(_, Region::Underwater) => self
                .get_site_at_square(state)
                .is_some_and(|site| site.is_water_site(state).unwrap_or_default()),
            Location::Intersection(squares, region) => squares
                .iter()
                .all(|square| Location::Square(*square, region.clone()).is_location(state)),
        }
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
        let mut nearby = self.get_adjacent();
        let region = self.region().clone();

        match self {
            Location::Square(square, _) => {
                let diagonals = match square % 5 {
                    0 => vec![
                        Location::Square(square.saturating_add(4), region.clone()),
                        Location::Square(square.saturating_sub(6), region.clone()),
                    ],
                    1 => vec![
                        Location::Square(square.saturating_sub(4), region.clone()),
                        Location::Square(square.saturating_add(6), region.clone()),
                    ],
                    _ => vec![
                        Location::Square(square.saturating_sub(4), region.clone()),
                        Location::Square(square.saturating_add(6), region.clone()),
                        Location::Square(square.saturating_add(4), region.clone()),
                        Location::Square(square.saturating_sub(6), region.clone()),
                    ],
                };
                nearby.extend(diagonals);
                nearby.retain(|location| location.square().unwrap_or(0) > 0);
                nearby.retain(|location| location.square().unwrap_or(0) <= 20);
                nearby.dedup();
                nearby
            }
            Location::Intersection(squares, _) => {
                nearby.clear();
                for square in squares {
                    let square_location = Location::Square(*square, region.clone());
                    nearby.extend(square_location.get_adjacent());

                    let diagonals = match square % 5 {
                        0 => vec![
                            Location::Square(square.saturating_add(4), region.clone()),
                            Location::Square(square.saturating_sub(6), region.clone()),
                        ],
                        1 => vec![
                            Location::Square(square.saturating_sub(4), region.clone()),
                            Location::Square(square.saturating_add(6), region.clone()),
                        ],
                        _ => vec![
                            Location::Square(square.saturating_sub(4), region.clone()),
                            Location::Square(square.saturating_add(6), region.clone()),
                            Location::Square(square.saturating_add(4), region.clone()),
                            Location::Square(square.saturating_sub(6), region.clone()),
                        ],
                    };
                    nearby.extend(diagonals);
                }

                for intersection in Location::all_intersections() {
                    if let Location::Intersection(intersection_squares, _) = &intersection
                        && intersection_squares != squares
                        && intersection_squares
                            .iter()
                            .any(|square| squares.contains(square))
                    {
                        nearby.push(Location::Intersection(
                            intersection_squares.clone(),
                            region.clone(),
                        ));
                    }
                }

                nearby.dedup();
                nearby
            }
        }
    }

    pub fn is_nearby(&self, other: &Location) -> bool {
        self.get_nearby().contains(other)
    }

    pub fn is_adjacent(&self, other: &Location) -> bool {
        self.get_adjacent().contains(other)
    }

    pub fn steps_to_location(&self, other: &Location) -> Option<u8> {
        self.min_steps_to_location(other)
    }

    pub fn min_steps_to_location(&self, other: &Location) -> Option<u8> {
        let mut visited = Vec::new();
        let mut to_visit = vec![(self.clone(), 0)];

        while let Some((current_location, current_step)) = to_visit.pop() {
            if &current_location == other {
                return Some(current_step);
            }

            if !visited.contains(&current_location) {
                visited.push(current_location.clone());

                for adjacent in current_location.get_adjacent() {
                    to_visit.push((adjacent, current_step + 1));
                }
            }
        }

        None
    }

    pub fn get_nearby_locations(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter(|location| location.is_location(state))
            .collect()
    }

    pub fn get_adjacent_locations(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter(|location| location.is_location(state))
            .collect()
    }

    pub fn get_nearby_sites(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let site_location = Location::Square(square, Region::Surface);
                site_location
                    .get_site(state)
                    .is_some()
                    .then_some(site_location)
            })
            .collect()
    }

    pub fn get_adjacent_sites(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let site_location = Location::Square(square, Region::Surface);
                site_location
                    .get_site(state)
                    .is_some()
                    .then_some(site_location)
            })
            .collect()
    }

    pub fn get_nearby_voids(&self, state: &State) -> Vec<Self> {
        self.get_nearby()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let void = Location::Square(square, Region::Void);
                void.is_location(state).then_some(void)
            })
            .collect()
    }

    pub fn get_adjacent_voids(&self, state: &State) -> Vec<Self> {
        self.get_adjacent()
            .into_iter()
            .filter_map(|location| {
                let square = location.get_square()?;
                let void = Location::Square(square, Region::Void);
                void.is_location(state).then_some(void)
            })
            .collect()
    }

    pub fn steps_in_direction(
        &self,
        direction: &Direction,
        steps: u8,
        state: &State,
        source_card_id: Option<&CardId>,
    ) -> Option<Self> {
        let mut current_zone = self.clone();
        for _ in 0..steps {
            match current_zone.step_in_direction(direction, state, source_card_id) {
                Some(z) => current_zone = z,
                None => return None,
            }
        }
        Some(current_zone)
    }

    pub fn step_in_direction(
        &self,
        direction: &Direction,
        state: &State,
        source_card_id: Option<&CardId>,
    ) -> Option<Self> {
        match self {
            Location::Square(square, region) => Some(Location::Square(
                Self::square_after_steps_in_direction(
                    *square,
                    direction,
                    1,
                    state,
                    source_card_id,
                )?,
                region.clone(),
            )),
            Location::Intersection(locs, region) => {
                let new_squares: Vec<u8> = locs
                    .iter()
                    .filter_map(|sq| {
                        Self::square_after_steps_in_direction(
                            *sq,
                            direction,
                            1,
                            state,
                            source_card_id,
                        )
                    })
                    .collect();
                if new_squares.len() != locs.len() {
                    return None;
                }

                for intersection in Location::all_intersections() {
                    if let Location::Intersection(intersection_squares, _) = &intersection
                        && intersection_squares == &new_squares
                    {
                        return Some(Location::Intersection(new_squares, region.clone()));
                    }
                }

                None
            }
        }
    }

    pub fn occupied_regions(&self, state: &State) -> Vec<Region> {
        match self {
            Location::Square(square, region) => {
                vec![Self::occupied_square_region(*square, region, state)]
            }
            Location::Intersection(squares, region) => {
                let mut regions = squares
                    .iter()
                    .map(|square| Self::occupied_square_region(*square, region, state))
                    .collect::<Vec<_>>();
                regions.sort();
                regions.dedup();
                regions
            }
        }
    }

    fn occupied_square_region(square: u8, region: &Region, state: &State) -> Region {
        let location = Location::Square(square, region.clone());
        let Some(site) = location.get_site_at_square(state) else {
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

    pub fn is_valid_play_location_for(
        &self,
        state: &State,
        card_id: &CardId,
        player_id: &PlayerId,
    ) -> anyhow::Result<bool> {
        let should_override = state
            .active_continuous_effects()
            .into_iter()
            .filter(|e| match e {
                OngoingEffect::OverrideValidPlayZone {
                    affected_locations,
                    affected_cards,
                    ..
                } => {
                    affected_locations.options(state).contains(self)
                        && affected_cards.matches(card_id, state)
                }
                _ => false,
            })
            .count()
            > 0;
        if should_override {
            return Ok(true);
        }

        match self {
            Location::Square(_, _) => {
                let card = state.get_card(card_id);
                // Auras should be played on intersections unless otherwise stated.
                if card.get_card_type() == CardType::Aura {
                    return Ok(false);
                }

                let site_in_zone = self.get_site(state);
                if let Some(site) = site_in_zone {
                    println!("Is valid play site for");
                    return site.is_valid_play_site_for(state, card_id, player_id);
                }

                // If there's no site in the zone, only cards with Voidwalk can be played there.
                match card.get_card_type() {
                    CardType::Site => {
                        let avatar_id = state.get_player_avatar_id(player_id)?;
                        let avatar = state.get_card(&avatar_id);
                        let avatar_location = avatar.get_location();
                        if avatar_location.get_site(state).is_none() {
                            return Ok(avatar_location == self);
                        }

                        let empty_adjacent_zones: Vec<Location> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(player_id)
                            .not_named(Rubble::NAME.to_string())
                            .all(state)
                            .into_iter()
                            .map(|cid| state.get_card(&cid).get_location())
                            .flat_map(|l| l.get_adjacent())
                            .filter(|l| l.get_site(state).is_none())
                            .collect();

                        Ok(empty_adjacent_zones.contains(self))
                    }
                    _ => Ok(card.has_ability(state, &Ability::Voidwalk)),
                }
            }
            Location::Intersection(sqs, region) => {
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
                            .filter_map(|cid| state.get_card(&cid).get_location().square())
                            .collect();
                        Ok(sqs.iter().any(|sq| site_squares.contains(sq)))
                    }
                    CardType::Aura => Ok(matches!(region, Region::Surface | Region::Void)),
                    _ => Ok(false),
                }
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

    pub fn is_in_play(&self) -> bool {
        matches!(
            self,
            Zone::Location(Location::Square(_, _)) | Zone::Location(Location::Intersection(_, _))
        )
    }
}
