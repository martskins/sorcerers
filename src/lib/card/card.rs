use crate::{
    card::CARD_CONSTRUCTORS,
    effect::{Counter, Effect, ModifierCounter, TokenType},
    game::{
        AvatarAction, CardAction, Direction, Element, PlayerId, Thresholds, UnitAction, are_adjacent, are_nearby,
        get_adjacent_zones, get_nearby_zones,
    },
    query::{CardQuery, ZoneQuery},
    state::State,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Avatar,
    Minion,
    Magic,
    Artifact,
    Aura,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edition {
    Alpha,
    Beta,
    ArthurianLegends,
    Dragonlord,
    Gothic,
}

impl Edition {
    pub fn url_name(&self) -> &str {
        match self {
            Edition::Alpha => "alp",
            Edition::Beta => "bet",
            Edition::ArthurianLegends => "art",
            Edition::Dragonlord => "drg",
            Edition::Gothic => "got",
        }
    }
}

#[derive(Debug, PartialOrd, Ord, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub enum Plane {
    None,
    Void,
    Underground,
    Submerged,
    Surface,
    Air,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub enum Zone {
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Realm(u8),
    Cemetery,
    Banish,
    Intersection(Vec<u8>),
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::None => write!(f, "None"),
            Zone::Hand => write!(f, "Hand"),
            Zone::Spellbook => write!(f, "Spellbook"),
            Zone::Atlasbook => write!(f, "Atlasbook"),
            Zone::Realm(sq) => write!(f, "{}", sq),
            Zone::Cemetery => write!(f, "Cemetery"),
            Zone::Banish => write!(f, "Banish"),
            Zone::Intersection(locs) => write!(
                f,
                "Intersection of ({})",
                locs.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(",")
            ),
        }
    }
}

impl Zone {
    pub fn is_in_play(&self) -> bool {
        match self {
            Zone::Realm(_) | Zone::Intersection(_) => true,
            _ => false,
        }
    }

    pub fn all_intersections() -> Vec<Zone> {
        vec![
            Zone::Intersection(vec![1, 2, 6, 7]),
            Zone::Intersection(vec![2, 3, 7, 8]),
            Zone::Intersection(vec![3, 4, 8, 9]),
            Zone::Intersection(vec![4, 5, 9, 10]),
            Zone::Intersection(vec![6, 7, 11, 12]),
            Zone::Intersection(vec![7, 8, 12, 13]),
            Zone::Intersection(vec![8, 9, 13, 14]),
            Zone::Intersection(vec![9, 10, 14, 15]),
            Zone::Intersection(vec![11, 12, 16, 17]),
            Zone::Intersection(vec![12, 13, 17, 18]),
            Zone::Intersection(vec![13, 14, 18, 19]),
            Zone::Intersection(vec![14, 15, 19, 20]),
        ]
    }

    pub fn all_realm() -> Vec<Zone> {
        (1..=20).map(|sq| Zone::Realm(sq)).collect()
    }

    pub fn get_square(&self) -> Option<u8> {
        match self {
            Zone::Realm(sq) => Some(*sq),
            _ => None,
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

    pub fn get_minions<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        state
            .get_cards_in_zone(self)
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| {
                if let Some(owner_id) = owner_id {
                    c.get_owner_id() == owner_id
                } else {
                    true
                }
            })
            .cloned()
            .collect::<Vec<&Box<dyn Card>>>()
    }

    pub fn get_units<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        state
            .get_cards_in_zone(self)
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| {
                if let Some(owner_id) = owner_id {
                    c.get_owner_id() == owner_id
                } else {
                    true
                }
            })
            .cloned()
            .collect::<Vec<&Box<dyn Card>>>()
    }

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        state
            .get_cards_in_zone(self)
            .iter()
            .find(|c| c.is_site())
            .map_or(None, |c| c.get_site())
            .clone()
    }

    pub fn get_nearby_units<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        get_nearby_zones(self)
            .iter()
            .flat_map(|z| {
                state
                    .get_cards_in_zone(z)
                    .iter()
                    .filter(|c| c.is_unit())
                    .filter(|c| {
                        if let Some(owner_id) = owner_id {
                            c.get_owner_id() == owner_id
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect::<Vec<&Box<dyn Card>>>()
            })
            .collect()
    }

    pub fn get_nearby_sites<'a>(&self, state: &'a State, owner_id: Option<&uuid::Uuid>) -> Vec<&'a Box<dyn Card>> {
        get_nearby_zones(self)
            .iter()
            .flat_map(|z| {
                state
                    .get_cards_in_zone(z)
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| {
                        if let Some(owner_id) = owner_id {
                            c.get_owner_id() == owner_id
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect::<Vec<&Box<dyn Card>>>()
            })
            .collect()
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
            Zone::Realm(square) => {
                let zone = match direction {
                    Direction::Up => Zone::Realm(square.saturating_add(5)),
                    Direction::Down => Zone::Realm(square.saturating_sub(5)),
                    Direction::Left => Zone::Realm(square.saturating_sub(1)),
                    Direction::Right => Zone::Realm(square.saturating_add(1)),
                    Direction::TopLeft => Zone::Realm(square.saturating_add(4)),
                    Direction::TopRight => Zone::Realm(square.saturating_add(6)),
                    Direction::BottomLeft => Zone::Realm(square.saturating_sub(6)),
                    Direction::BottomRight => Zone::Realm(square.saturating_sub(4)),
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
            Zone::Intersection(locs) => {
                let new_squares: Vec<u8> = locs
                    .iter()
                    .filter_map(|sq| {
                        let realm_zone = Zone::Realm(*sq);
                        realm_zone.zone_in_direction(direction, 1)?.get_square()
                    })
                    .collect();

                for intersection in Zone::all_intersections() {
                    if let Zone::Intersection(locs) = &intersection {
                        if locs == &new_squares {
                            return Some(intersection);
                        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub edition: Edition,
    pub zone: Zone,
    pub plane: Plane,
    pub card_type: CardType,
    pub modifiers: Vec<Ability>,
    pub damage_taken: u8,
    pub attached_to: Option<uuid::Uuid>,
    pub rarity: Rarity,
    pub num_arts: usize,
}

impl CardData {
    pub fn is_site(&self) -> bool {
        self.card_type == CardType::Site
    }

    pub fn is_spell(&self) -> bool {
        !self.is_site()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_token(&self) -> bool {
        self.rarity == Rarity::Token
    }

    pub fn get_edition(&self) -> &Edition {
        &self.edition
    }
}

pub trait CloneBoxedCard {
    fn clone_box(&self) -> Box<dyn Card>;
}

impl<T> CloneBoxedCard for T
where
    T: 'static + Card + Clone,
{
    fn clone_box(&self) -> Box<dyn Card> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub enum AdditionalCost {
    Tap { card: CardQuery, count: usize },
    Discard { card: CardQuery, count: usize },
    Sacrifice { card: CardQuery, count: usize },
}

#[derive(Debug, Clone, Default)]
pub struct Cost {
    pub mana: u8,
    pub thresholds: Thresholds,
    pub additional: Vec<AdditionalCost>,
}

impl Cost {
    pub fn new(mana: u8, thresholds: impl Into<Thresholds>) -> Self {
        Self {
            mana,
            thresholds: thresholds.into(),
            additional: vec![],
        }
    }

    pub fn zero() -> Self {
        Self {
            mana: 0,
            thresholds: Thresholds::new(),
            additional: vec![],
        }
    }

    pub fn can_afford(&self, state: &State, player_id: &PlayerId) -> anyhow::Result<bool> {
        let resources = state.get_player_resources(player_id)?;
        let has_resources = resources.mana >= self.mana
            && resources.thresholds.fire >= self.thresholds.fire
            && resources.thresholds.air >= self.thresholds.air
            && resources.thresholds.earth >= self.thresholds.earth
            && resources.thresholds.water >= self.thresholds.water;

        if !has_resources {
            return Ok(false);
        }

        for other in &self.additional {
            match other {
                AdditionalCost::Tap { card, count } => {
                    if card.options(state).len() < *count {
                        return Ok(false);
                    }
                }
                AdditionalCost::Discard { card, count } => {
                    if card.options(state).len() < *count {
                        return Ok(false);
                    }
                }
                AdditionalCost::Sacrifice { card, count } => {
                    if card.options(state).len() < *count {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }
}

// The `Card` trait defines the core interface for all cards in Sorcerers.
// It provides methods for accessing card properties, handling game logic, and interacting with the game state.
//
// Card represents all types of cards in the game, including avatars, sites, minions, etc.
// Implementors should override relevant methods for their specific card type.
#[async_trait::async_trait]
pub trait Card: Debug + Send + Sync + CloneBoxedCard {
    fn get_name(&self) -> &str;
    fn get_base(&self) -> &CardBase;
    fn get_base_mut(&mut self) -> &mut CardBase;

    fn get_edition(&self) -> &Edition {
        &self.get_base().edition
    }

    fn is_tapped(&self) -> bool {
        self.get_base().tapped
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.get_base().id
    }

    // When resolving a CardQuery, this method allows the card to override the query. A useful
    // usecase for this method is for example overriding the valid targets of a spell when there's
    // a card in play that affects targeting.
    fn card_query_override(&self, _state: &State, _query: &CardQuery) -> anyhow::Result<Option<CardQuery>> {
        Ok(None)
    }

    // When resolving a ZoneQuery, this method allows the card to override the query. A useful
    // usecase for this method is for example overriding the zones that the player can pick from
    // when the there's a card in play that affects zone selection.
    fn zone_query_override(&self, _state: &State, _query: &ZoneQuery) -> anyhow::Result<Option<ZoneQuery>> {
        Ok(None)
    }

    // When resolving an effect, this methods allows a card in play to replace that event with a
    // different set of effects.
    fn replace_effect(&self, _state: &State, _effect: &Effect) -> Option<Vec<Effect>> {
        None
    }

    // Removes the power counter with the given ID from the card.
    fn remove_power_counter(&mut self, id: &uuid::Uuid) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.power_counters.retain(|c| &c.id != id);
        }
    }

    // Removes the modifier counter with the given ID from the card.
    fn remove_modifier_counter(&mut self, id: &uuid::Uuid) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifier_counters.retain(|c| &c.id != id);
        }
    }

    // Returns the ID of the player who owns this card.
    fn get_owner_id(&self) -> &PlayerId {
        &self.get_base().owner_id
    }

    // Returns the ID of the player who controls this card.
    fn get_controller_id(&self) -> &PlayerId {
        &self.get_base().controller_id
    }

    // Returns a list of effects that must be applied after this card attacks.
    async fn after_attack(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    // Returns a list of effects that must be applied when this card is defending against an
    // attack.
    fn on_defend(&self, state: &State, attacker_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        if let Some(power) = self.get_power(&state)? {
            return Ok(vec![Effect::TakeDamage {
                card_id: attacker_id.clone(),
                from: self.get_id().clone(),
                damage: power,
            }]);
        }

        Ok(vec![])
    }

    // Sets custom data for the card. By default, this method returns an error indicating that
    // the operation is not implemented for the specific card type.
    // If a card needs to hold specific data, and you need to modify it, override this method with
    // a method that downcasts the data to the appropriate type and sets it on the card.
    fn set_data(&mut self, _data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("set_data not implemented for {}", self.get_name()))
    }

    // Returns the zones that are within the given steps of the specified zone, using this card as
    // the reference for movement capabilities.
    fn get_zones_within_steps_of(&self, state: &State, steps: u8, zone: &Zone) -> Vec<Zone> {
        let mut visited = Vec::new();
        let mut to_visit = vec![(zone.clone(), 0)];

        while let Some((current_zone, current_step)) = to_visit.pop() {
            if current_step > steps {
                continue;
            }

            if !visited.contains(&current_zone) {
                visited.push(current_zone.clone());

                if self.has_modifier(state, &Ability::Airborne) {
                    for nearby in current_zone.get_nearby() {
                        to_visit.push((nearby, current_step + 1));
                    }
                } else {
                    for adjacent in current_zone.get_adjacent() {
                        to_visit.push((adjacent, current_step + 1));
                    }
                }
            }
        }

        if self.is_unit() && !self.has_modifier(state, &Ability::Voidwalk) {
            visited = visited
                .iter()
                .filter(|z| z.get_site(state).is_some())
                .cloned()
                .collect();
        }

        visited
    }

    // Returns the zones that are within the given steps of this card's current zone.
    fn get_zones_within_steps(&self, state: &State, steps: u8) -> Vec<Zone> {
        self.get_zones_within_steps_of(state, steps, self.get_zone())
    }

    // Base take damage behaviour for cards. This method MUST NOT BE OVERRIDEN by specific card
    // types. Instead, specific card types should override `on_take_damage`, and can use
    // base_take_damage to get the default behaviour.
    fn base_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> anyhow::Result<Vec<Effect>> {
        if self.is_unit() {
            let ub = self
                .get_unit_base_mut()
                .ok_or(anyhow::anyhow!("unit card has no unit base"))?;
            ub.damage += damage;

            let attacker = state.get_card(from);
            if ub.damage >= self.get_toughness(state).unwrap_or(0) || attacker.has_modifier(state, &Ability::Lethal) {
                return Ok(vec![Effect::bury_card(self.get_id(), self.get_zone())]);
            }

            return Ok(vec![]);
        } else if self.is_site() {
            let avatar_id = state.get_player_avatar_id(self.get_controller_id())?;
            return Ok(vec![Effect::TakeDamage {
                card_id: avatar_id,
                from: from.clone(),
                damage,
            }]);
        }

        Ok(vec![])
    }

    // Base on-summon behaviour for site cards. This method MUST NOT BE OVERRIDEN by specific card
    // implementations. Instead, specific card types should override `on_summon`, and can use
    // base_site_on_summon to get the default behaviour.
    fn base_site_on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let site_base = self
            .get_site_base()
            .ok_or(anyhow::anyhow!("site card has no site base"))?;
        Ok(vec![Effect::AddResources {
            player_id: self.get_owner_id().clone(),
            mana: site_base.provided_mana,
            thresholds: site_base.provided_thresholds.clone(),
        }])
    }

    fn default_get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        match self.get_card_type() {
            CardType::Artifact => Ok(vec![]),
            CardType::Aura => Ok(Zone::all_intersections()
                .iter()
                .filter(|z| match z {
                    Zone::Intersection(sqs) => sqs.iter().any(|_| state.cards.iter().any(|c| c.is_site())),
                    _ => false,
                })
                .cloned()
                .collect()),
            CardType::Minion => Ok(state
                .cards
                .iter()
                .filter(|c| c.get_owner_id() == self.get_owner_id())
                .filter(|c| c.is_site())
                .filter_map(|c| match c.get_zone() {
                    z @ Zone::Realm(_) => Some(z),
                    _ => None,
                })
                .cloned()
                .collect()),
            CardType::Site => {
                let player_id = self.get_owner_id();
                let has_played_site = state
                    .cards
                    .iter()
                    .any(|c| c.get_owner_id() == player_id && c.is_site() && matches!(c.get_zone(), Zone::Realm(_)));
                if !has_played_site {
                    let avatar = state
                        .cards
                        .iter()
                        .find(|c| c.get_owner_id() == player_id && c.is_avatar())
                        .ok_or(anyhow::anyhow!("player has no avatar"))?;
                    match avatar.get_zone() {
                        z @ Zone::Realm(_) => return Ok(vec![z.clone()]),
                        _ => return Err(anyhow::anyhow!("Avatar not in realm")),
                    }
                }

                let sites: Vec<&Zone> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter_map(|c| match c.get_zone() {
                        z @ Zone::Realm(_) => Some(z),
                        _ => None,
                    })
                    .collect();

                let occupied_squares: Vec<&Zone> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| c.is_site())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .flat_map(|c| match c.get_zone() {
                        z @ Zone::Realm(_) => vec![z],
                        _ => vec![],
                    })
                    .collect();

                Ok(occupied_squares
                    .iter()
                    .flat_map(|c| get_adjacent_zones(c))
                    .filter(|c| !occupied_squares.contains(&c))
                    .filter(|c| !sites.contains(&c))
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }

    // Retuns the plane the card is currently on. If the card is not in a zone with a site, it is
    // in the void.
    fn get_plane(&self, state: &State) -> &Plane {
        if self.get_zone().get_site(state).is_none() {
            return &Plane::Void;
        }

        &self.get_base().plane
    }

    // Returns the amount of damage taken by the card. Defaults to 0 for non-unit cards.
    fn get_damage_taken(&self) -> anyhow::Result<u8> {
        if self.is_unit() {
            return Ok(self
                .get_unit_base()
                .ok_or(anyhow::anyhow!("unit card has no unit base"))?
                .damage);
        }

        Ok(0)
    }

    // Returns the type of the card.
    fn get_card_type(&self) -> CardType {
        if self.is_site() {
            CardType::Site
        } else if self.is_avatar() {
            CardType::Avatar
        } else if self.is_aura() {
            CardType::Aura
        } else if self.is_unit() {
            CardType::Minion
        } else if self.is_artifact() {
            CardType::Artifact
        } else {
            CardType::Magic
        }
    }

    // Returns the valid zones where this card can be played in.
    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        self.default_get_valid_play_zones(state)
    }

    fn is_ranged(&self, state: &State) -> anyhow::Result<bool> {
        for modif in self.get_modifiers(state)? {
            if let Ability::Ranged(_) = modif {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Returns whether the card has the given modifier.
    fn has_modifier(&self, _state: &State, modifier: &Ability) -> bool {
        if self
            .get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifiers
            .contains(&modifier)
        {
            return true;
        }

        self.get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifier_counters
            .iter()
            .find(|c| &c.modifier == modifier)
            .is_some()
    }

    // Returns the elements associated to this card.
    fn get_elements(&self, state: &State) -> anyhow::Result<Vec<Element>> {
        let mut elements = Vec::new();
        let thresholds = self.get_cost(state)?.thresholds;
        if thresholds.fire > 0 {
            elements.push(Element::Fire);
        }
        if thresholds.water > 0 {
            elements.push(Element::Water);
        }
        if thresholds.earth > 0 {
            elements.push(Element::Earth);
        }
        if thresholds.air > 0 {
            elements.push(Element::Air);
        }

        Ok(elements)
    }

    // Returns all valid move paths from the card's current zone to the given zone. The paths
    // include the starting, ending and all intermediate zones.
    fn get_valid_move_paths(&self, state: &State, to: &Zone) -> anyhow::Result<Vec<Vec<Zone>>> {
        let from = self.get_zone().clone();
        let valid_zones = self.get_valid_move_zones(state)?;
        if !valid_zones.contains(&to) {
            return Ok(vec![]);
        }

        let max_steps = self.get_steps_per_movement(state)?;
        let is_traversable =
            |current: &Zone, next: &Zone| self.get_zones_within_steps_of(state, 1, current).contains(next);

        let mut paths = Vec::new();
        let mut queue: Vec<(Vec<Zone>, Zone)> = vec![(vec![from.clone()], from.clone())];
        while let Some((path, current)) = queue.pop() {
            if &current == to {
                if path.len() - 1 <= max_steps.into() {
                    paths.push(path.clone());
                }
                continue;
            }
            if path.len() - 1 >= max_steps.into() {
                continue;
            }
            for next in valid_zones.iter() {
                if path.contains(next) || &current == next {
                    continue;
                }
                if is_traversable(&current, next) {
                    let mut new_path = path.clone();
                    new_path.push(next.clone());
                    queue.push((new_path, next.clone()));
                }
            }
        }
        Ok(paths)
    }

    // Returns the number of steps this card can move per movement action.
    fn get_steps_per_movement(&self, state: &State) -> anyhow::Result<u8> {
        if self.has_modifier(state, &Ability::Immobile) {
            return Ok(0);
        }

        let extra_steps: u8 = self
            .get_modifiers(state)?
            .into_iter()
            .map(|m| match m {
                Ability::Movement(s) => s,
                _ => 0,
            })
            .sum();

        Ok(extra_steps + 1)
    }

    fn base_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok(self
            .get_zones_within_steps(state, self.get_steps_per_movement(state)?)
            .iter()
            .filter(|z| {
                // If the card is not a unit, it might be an aura, in which case the result of
                // get_zones_within_steps should be returned as is.
                if !self.is_unit() {
                    return true;
                }

                if self.has_modifier(state, &Ability::Voidwalk) {
                    return true;
                }

                z.get_site(state).map_or(false, |c| {
                    c.can_be_entered_by(self.get_id(), self.get_zone(), self.get_plane(state), state)
                })
            })
            .cloned()
            .collect())
    }

    // Returns the valid zones this card can move to from its current zone.
    fn get_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        self.base_valid_move_zones(state)
    }

    // Returns the valid attack targets for this card.
    fn get_valid_attack_targets_from_zone(&self, state: &State, ranged: bool, zone: &Zone) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() != self.get_owner_id())
            .filter(|c| c.is_unit() || c.is_site())
            .filter(|c| c.can_be_targetted_by(state, self.get_controller_id()))
            .filter(|c| {
                let same_plane = c.get_base().plane == self.get_base().plane;
                let ranged_on_airborne =
                    ranged && self.get_base().plane == Plane::Surface && c.get_base().plane == Plane::Air;
                let airborne_on_surface = self.get_base().plane == Plane::Air && c.get_base().plane == Plane::Surface;
                return same_plane || ranged_on_airborne || airborne_on_surface;
            })
            .filter(|_| {
                let attacker_is_airborne = self.has_modifier(state, &Ability::Airborne);
                if !attacker_is_airborne {
                    return zone.is_adjacent(&self.get_zone());
                }

                return zone.is_nearby(&self.get_zone());
            })
            .map(|c| c.get_id().clone())
            .collect()
    }

    // Returns the valid attack targets for this card.
    fn get_valid_attack_targets(&self, state: &State, ranged: bool) -> Vec<uuid::Uuid> {
        self.get_valid_attack_targets_from_zone(state, ranged, self.get_zone())
    }

    // Returns the toughness of the card. Returns None for non-unit cards.
    fn get_toughness(&self, _state: &State) -> Option<u8> {
        match self.get_unit_base() {
            Some(base) => {
                let mut toughness = base.toughness;
                for counter in &base.power_counters {
                    toughness = toughness.saturating_add_signed(counter.toughness);
                }
                Some(toughness)
            }
            None => None,
        }
    }

    // Returns all modifiers currently applied to the card.
    fn get_modifiers(&self, state: &State) -> anyhow::Result<Vec<Ability>> {
        Ok(self.base_get_modifiers(state))
    }

    fn base_get_modifiers(&self, state: &State) -> Vec<Ability> {
        match self.get_unit_base() {
            Some(base) => {
                let mut modifiers = base.modifiers.clone();
                for counter in &base.modifier_counters {
                    modifiers.push(counter.modifier.clone());
                }

                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let mods = card.area_modifiers(state);
                    if let Some(mods) = mods.grants_abilities.get(self.get_id()) {
                        modifiers.extend(mods.clone());
                    }
                }

                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let mods = card.area_modifiers(state);
                    if let Some(mods) = mods.removes_abilities.get(self.get_id()) {
                        for modif in mods {
                            modifiers.retain(|m| m != modif);
                        }
                    }
                }

                modifiers
            }
            None => vec![],
        }
    }

    fn base_get_power(&self, _state: &State) -> Option<u8> {
        match self.get_unit_base() {
            Some(base) => {
                let mut power = base.power;
                for counter in &base.power_counters {
                    power = power.saturating_add_signed(counter.power);
                }
                Some(power)
            }
            None => None,
        }
    }

    fn get_power(&self, state: &State) -> anyhow::Result<Option<u8>> {
        Ok(self.base_get_power(state))
    }

    fn get_cost(&self, state: &State) -> anyhow::Result<Cost> {
        let mut cost = self.get_base().cost.clone();
        cost.additional = self.get_additional_costs(state)?;
        Ok(cost)
    }

    fn get_additional_costs(&self, _state: &State) -> anyhow::Result<Vec<AdditionalCost>> {
        Ok(vec![])
    }

    // Returns the avatar base if the card is an avatar, None otherwise.
    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        None
    }

    // Returns a mutable reference to the avatar base if the card is an avatar, None otherwise.
    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        None
    }

    // Returns the site base if the card is a site, None otherwise.
    fn get_site_base(&self) -> Option<&SiteBase> {
        None
    }

    // Upcasts a card to a site trait object if it is a site, None otherwise.
    fn get_site(&self) -> Option<&dyn Site> {
        None
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        None
    }

    fn get_aura_base(&self) -> Option<&AuraBase> {
        None
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        None
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        None
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        None
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        None
    }

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        None
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        None
    }

    fn get_zone(&self) -> &Zone {
        &self.get_base().zone
    }

    fn set_zone(&mut self, zone: Zone) {
        self.get_base_mut().zone = zone;
    }

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn deathrite(&self, _state: &State, from: &Zone) -> Vec<Effect> {
        if self.is_site() && from.is_in_play() {
            return vec![
                Effect::SummonToken {
                    player_id: self.get_controller_id().clone(),
                    token_type: TokenType::Rubble,
                    zone: from.clone(),
                },
                Effect::RemoveResources {
                    player_id: self.get_owner_id().clone(),
                    mana: 0,
                    thresholds: self.get_site_base().unwrap().provided_thresholds.clone(),
                },
            ];
        }

        vec![]
    }

    fn can_be_targetted_by(&self, state: &State, player_id: &PlayerId) -> bool {
        if self.has_modifier(state, &Ability::Stealth) && self.get_owner_id() != player_id {
            return false;
        }

        true
    }

    fn is_token(&self) -> bool {
        false
    }

    fn is_site(&self) -> bool {
        self.get_site_base().is_some()
    }

    fn is_avatar(&self) -> bool {
        self.get_avatar_base().is_some()
    }

    fn is_artifact(&self) -> bool {
        self.get_artifact_base().is_some()
    }

    fn is_unit(&self) -> bool {
        self.get_unit_base().is_some()
    }

    fn is_minion(&self) -> bool {
        self.is_unit() && !self.is_avatar()
    }

    fn is_aura(&self) -> bool {
        self.get_aura_base().is_some()
    }

    fn can_cast(&self, state: &State, spell: &Box<dyn Card>) -> anyhow::Result<bool> {
        if !self.get_zone().is_in_play() {
            return Ok(false);
        }

        if self.get_owner_id() != spell.get_owner_id() {
            return Ok(false);
        }

        if self.is_avatar() {
            return Ok(true);
        }

        let elements = spell.get_elements(state)?;
        for element in elements {
            if self.has_modifier(state, &Ability::Spellcaster(element)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn on_move(&self, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        let mut all_effects = Vec::new();
        for card in &state.cards {
            for modif in card.get_modifiers(state)? {
                let effects = modif.on_move(self.get_id(), state, &path)?;
                all_effects.extend(effects);
            }
        }

        Ok(all_effects)
    }

    async fn on_visit_zone(&self, _state: &State, _to: &Zone) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> anyhow::Result<Vec<Effect>> {
        self.base_take_damage(state, from, damage)
    }

    async fn on_turn_start(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    async fn on_turn_end(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn remove_modifier(&mut self, modifier: &Ability) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.retain(|a| a != modifier);
        }
    }

    fn add_modifier(&mut self, modifier: Ability) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.push(modifier);
        }
    }

    fn on_summon(&mut self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if self.is_site() {
            return self.base_site_on_summon(state);
        }

        Ok(vec![])
    }

    async fn on_cast(&mut self, _state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn base_avatar_actions(&self, state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        let mut actions: Vec<Box<dyn CardAction>> = self.base_unit_actions(state)?;
        actions.push(Box::new(AvatarAction::DrawSite));
        if state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| matches!(c.get_zone(), Zone::Hand))
            .count()
            > 0
        {
            actions.push(Box::new(AvatarAction::PlaySite));
        }

        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            let mods = card.area_modifiers(state);
            if let Some(mods) = mods.grants_actions.get(self.get_id()) {
                actions.extend(mods.clone());
            }
        }

        // TODO: should artifacts on avatars be able to provide actions?
        // let artifacts = state
        //     .cards
        //     .iter()
        //     .filter(|c| c.is_artifact())
        //     .filter_map(|c| c.get_artifact())
        //     .filter(|c| match c.get_bearer() {
        //         Ok(Some(bearer_id)) => bearer_id == *self.get_id(),
        //         _ => false,
        //     });
        // for artifact in artifacts {
        //     actions.extend(artifact.get_actions(state)?);
        // }

        Ok(actions)
    }

    fn base_unit_actions(&self, state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        let mut actions: Vec<Box<dyn CardAction>> = vec![Box::new(UnitAction::Attack), Box::new(UnitAction::Move)];
        if self.is_ranged(state)? {
            actions.push(Box::new(UnitAction::RangedAttack));
        }

        if self.has_modifier(state, &Ability::Burrowing) {
            actions.push(Box::new(UnitAction::Burrow));
        }

        if self.has_modifier(state, &Ability::Submerge) {
            actions.push(Box::new(UnitAction::Submerge));
        }

        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            let mods = card.area_modifiers(state);
            if let Some(mods) = mods.grants_actions.get(self.get_id()) {
                actions.extend(mods.clone());
            }
        }

        // TODO: should artifacts on units be able to provide actions?
        // let artifacts = state
        //     .cards
        //     .iter()
        //     .filter(|c| c.is_artifact())
        //     .filter_map(|c| c.get_artifact())
        //     .filter(|c| match c.get_bearer() {
        //         Ok(Some(bearer_id)) => bearer_id == *self.get_id(),
        //         _ => false,
        //     });
        // for artifact in artifacts {
        //     actions.extend(artifact.get_actions(state)?);
        // }

        Ok(actions)
    }

    // Returns the available actions for this card, given the current game state.
    fn get_actions(&self, state: &State) -> anyhow::Result<Vec<Box<dyn CardAction>>> {
        if self.is_avatar() {
            return Ok(self.base_avatar_actions(state)?);
        } else if self.is_unit() {
            return Ok(self.base_unit_actions(state)?);
        }

        Ok(vec![])
    }

    // Returns the modifiers that this card provides to other cards in the game.
    fn area_modifiers(&self, _state: &State) -> AreaModifiers {
        AreaModifiers::default()
    }

    fn area_effects(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn get_num_arts(&self) -> usize {
        1
    }
}

#[derive(Debug, Default, Clone)]
pub struct AreaModifiers {
    pub grants_abilities: HashMap<uuid::Uuid, Vec<Ability>>,
    pub removes_abilities: HashMap<uuid::Uuid, Vec<Ability>>,
    pub grants_actions: HashMap<uuid::Uuid, Vec<Box<dyn CardAction>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MinionType {
    Monster,
    Goblin,
    Ogre,
    Mortal,
    Demon,
    Dragon,
    Fairy,
    Beast,
    Spirit,
    Undead,
    Giant,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SiteType {
    Desert,
    Tower,
    Earth,
    Village,
}

#[derive(Debug, Default, Clone)]
pub struct SiteBase {
    pub provided_mana: u8,
    pub provided_thresholds: Thresholds,
    pub types: Vec<SiteType>,
}

pub trait Site: Card {
    fn provides(&self, element: &Element) -> anyhow::Result<u8> {
        let site_base = self.get_site_base().ok_or(anyhow::anyhow!("site card has no base"))?;

        let result = match element {
            Element::Fire => site_base.provided_thresholds.fire,
            Element::Earth => site_base.provided_thresholds.earth,
            Element::Air => site_base.provided_thresholds.air,
            Element::Water => site_base.provided_thresholds.water,
        };

        Ok(result)
    }

    fn on_card_enter(&self, _state: &State, _card_id: &uuid::Uuid) -> Vec<Effect> {
        vec![]
    }

    fn can_be_entered_by(&self, _card: &uuid::Uuid, _from: &Zone, _plane: &Plane, _state: &State) -> bool {
        true
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Ability {
    Disabled,
    Voidwalk,
    Airborne,
    Ranged(u8),
    Stealth,
    Lethal,
    Movement(u8),
    Burrowing,
    Landbound,
    Submerge,
    Spellcaster(Element),
    Charge,
    SummoningSickness,
    TakesNoDamageFromElement(Element),
    Immobile,
    Blaze(u8), // Specific modifier for the Blaze magic
}

impl Ability {
    fn on_move(&self, card_id: &uuid::Uuid, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        match self {
            Ability::Blaze(burn) => {
                if path.len() <= 1 {
                    return Ok(vec![]);
                }

                let mut effects = vec![];
                for zone in path {
                    if zone == path.last().unwrap() {
                        break;
                    }

                    let units = state.get_units_in_zone(&zone);
                    for unit in units {
                        let card = state.get_card(card_id);
                        effects.push(Effect::take_damage(unit.get_id(), card.get_id(), *burn));
                    }
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnitBase {
    pub power: u8,
    pub toughness: u8,
    pub modifiers: Vec<Ability>,
    pub damage: u8,
    pub power_counters: Vec<Counter>,
    pub modifier_counters: Vec<ModifierCounter>,
    pub types: Vec<MinionType>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum Rarity {
    Token,
    Ordinary,
    Exceptional,
    Elite,
    Unique,
}

#[derive(Debug, Clone)]
pub struct ArtifactBase {
    pub attached_to: Option<uuid::Uuid>,
}

pub trait Artifact: Card {
    fn get_valid_attach_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        match self.get_card_type() {
            CardType::Artifact => state
                .cards
                .iter()
                .filter(|c| c.is_unit())
                .filter(|c| c.get_controller_id() == self.get_owner_id())
                .map(|c| c.get_id().clone())
                .collect(),
            _ => vec![],
        }
    }

    fn get_bearer(&self) -> anyhow::Result<Option<uuid::Uuid>> {
        Ok(self
            .get_artifact_base()
            .ok_or(anyhow::anyhow!("artifact card has no base"))?
            .attached_to
            .clone())
    }
}

#[derive(Debug, Clone)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub zone: Zone,
    pub cost: Cost,
    pub plane: Plane,
    pub rarity: Rarity,
    pub edition: Edition,
    pub controller_id: PlayerId,
}

#[derive(Debug, Clone)]
pub struct AuraBase {}

pub trait Aura: Card {
    fn should_dispell(&self, _state: &State) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn get_affected_zones(&self, _state: &State) -> Vec<Zone> {
        match self.get_zone() {
            z @ Zone::Realm(_) => vec![z.clone()],
            Zone::Intersection(locs) => {
                let mut zones = Vec::new();
                for sq in locs {
                    zones.push(Zone::Realm(*sq));
                }
                zones
            }
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct AvatarBase {}

pub fn from_name(name: &str, player_id: &PlayerId) -> Box<dyn Card> {
    CARD_CONSTRUCTORS.get(name).unwrap()(player_id.clone())
}

pub fn from_name_and_zone(name: &str, player_id: &PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}

#[cfg(test)]
mod tests {
    use crate::{
        card::{Ability, ApprenticeWizard, Card, RimlandNomads, Zone},
        state::State,
    };

    #[test]
    fn test_get_valid_move_paths_movement_plus_1() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = RimlandNomads::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(card.clone()));

        let paths = card
            .get_valid_move_paths(&state, &Zone::Realm(14))
            .expect("paths to be computed");
        assert_eq!(paths.len(), 2, "Expected 2 paths, got {:?}", paths);
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(14)]));
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(14)]));
    }

    #[test]
    fn test_get_valid_move_paths_movement_plus_1_airborne() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = RimlandNomads::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        state.cards.push(Box::new(card.clone()));

        let paths = card
            .get_valid_move_paths(&state, &Zone::Realm(14))
            .expect("paths to be computed");
        assert_eq!(paths.len(), 3, "Expected 3 valid paths, got {:?}", paths);
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(14)]));
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(14)]));
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(14)]));
    }

    #[test]
    fn test_get_valid_move_paths_movement_plus_2() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = RimlandNomads::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(2));
        state.cards.push(Box::new(card.clone()));

        let paths = card
            .get_valid_move_paths(&state, &Zone::Realm(15))
            .expect("paths to be computed");
        assert_eq!(paths.len(), 3, "Expected 2 paths, got {:?}", paths);
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(10), Zone::Realm(15)]));
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(14), Zone::Realm(15)]));
        assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(14), Zone::Realm(15)]));
    }

    #[test]
    fn test_get_valid_move_zones_basic_movement() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(8),
            Zone::Realm(7),
            Zone::Realm(9),
            Zone::Realm(3),
            Zone::Realm(13),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_movement_plus_1() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(1));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(8),
            Zone::Realm(7),
            Zone::Realm(9),
            Zone::Realm(3),
            Zone::Realm(13),
            Zone::Realm(18),
            Zone::Realm(6),
            Zone::Realm(10),
            Zone::Realm(12),
            Zone::Realm(14),
            Zone::Realm(2),
            Zone::Realm(4),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_basic_movement_with_voids() {
        let zones_with_sites = vec![Zone::Realm(3), Zone::Realm(8), Zone::Realm(9)];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(3)];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_movement_plus_1_with_voids() {
        let zones_with_sites = vec![
            Zone::Realm(2),
            Zone::Realm(3),
            Zone::Realm(4),
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(12),
            Zone::Realm(13),
        ];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(1));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(2),
            Zone::Realm(3),
            Zone::Realm(4),
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(12),
            Zone::Realm(13),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_basic_movement_with_voidwalk() {
        let zones_with_sites = vec![Zone::Realm(3), Zone::Realm(8), Zone::Realm(9)];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Voidwalk);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(8),
            Zone::Realm(7),
            Zone::Realm(9),
            Zone::Realm(3),
            Zone::Realm(13),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_airborne() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(8),
            Zone::Realm(7),
            Zone::Realm(9),
            Zone::Realm(3),
            Zone::Realm(13),
            Zone::Realm(12),
            Zone::Realm(14),
            Zone::Realm(2),
            Zone::Realm(4),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_airborne_with_voids() {
        let zones_with_sites = vec![
            Zone::Realm(2),
            Zone::Realm(3),
            Zone::Realm(4),
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(12),
            Zone::Realm(13),
        ];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();

        let mut expected = vec![
            Zone::Realm(2),
            Zone::Realm(3),
            Zone::Realm(4),
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(12),
            Zone::Realm(13),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_get_valid_move_zones_airborne_and_voidwalk() {
        let zones_with_sites = vec![
            Zone::Realm(2),
            Zone::Realm(3),
            Zone::Realm(4),
            Zone::Realm(7),
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(12),
            Zone::Realm(13),
            Zone::Realm(14),
        ];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id.clone();
        let mut card = ApprenticeWizard::new(player_id.clone());
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        card.add_modifier(Ability::Voidwalk);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card.get_valid_move_zones(&state).expect("zones to be computed");
        zones.sort();
        let mut expected = vec![
            Zone::Realm(8),
            Zone::Realm(7),
            Zone::Realm(9),
            Zone::Realm(3),
            Zone::Realm(13),
            Zone::Realm(12),
            Zone::Realm(14),
            Zone::Realm(2),
            Zone::Realm(4),
        ];
        expected.sort();
        assert_eq!(zones, expected);
    }
}
