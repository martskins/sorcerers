pub mod beta;
pub mod foot_soldier;
pub mod frog;
pub mod rubble;
pub use beta::*;
pub use foot_soldier::*;
pub use frog::*;
pub use rubble::*;

use crate::{
    effect::{AbilityCounter, Counter, Effect, TokenType},
    game::{
        ActivatedAbility, AvatarAction, Direction, Element, PlayerId, Thresholds, UnitAction,
        are_adjacent, are_nearby, get_adjacent_zones, get_nearby_zones, pick_amount, pick_card,
        pick_option, pick_zone,
    },
    query::ZoneQuery,
    state::{CardQuery, ContinuousEffect, LoggedEffect, State, TemporaryEffect},
};
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, LazyLock},
};
use strum_macros::EnumIter;

pub type CardConstructor = fn(PlayerId) -> Box<dyn Card>;

#[distributed_slice]
pub static ALL_CARDS: [(&'static str, CardConstructor)];

pub static CARD_CONSTRUCTORS: LazyLock<HashMap<&'static str, CardConstructor>> =
    LazyLock::new(|| {
        let mut constructors = HashMap::new();
        for (name, constructor) in ALL_CARDS {
            constructors.insert(*name, *constructor);
        }
        constructors
    });

/// Returns true if a card with the given name exists in the registry.
pub fn card_exists(name: &str) -> bool {
    CARD_CONSTRUCTORS.contains_key(name)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Avatar,
    Minion,
    Magic,
    Artifact,
    Aura,
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardType::Site => write!(f, "Site"),
            CardType::Avatar => write!(f, "Avatar"),
            CardType::Minion => write!(f, "Minion"),
            CardType::Magic => write!(f, "Magic"),
            CardType::Artifact => write!(f, "Artifact"),
            CardType::Aura => write!(f, "Aura"),
        }
    }
}

impl CardType {
    pub fn is_unit(&self) -> bool {
        matches!(self, CardType::Minion | CardType::Avatar)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edition {
    #[default]
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

#[derive(Debug, Default, PartialOrd, Ord, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub enum Region {
    Void,
    Underground,
    Underwater,
    #[default]
    Surface,
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Void => write!(f, "Void"),
            Region::Underground => write!(f, "Underground"),
            Region::Underwater => write!(f, "Underwater"),
            Region::Surface => write!(f, "Surface"),
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
                locs.iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
        }
    }
}

impl Zone {
    pub fn is_in_play(&self) -> bool {
        matches!(self, Zone::Realm(_) | Zone::Intersection(_))
    }

    pub fn is_valid_play_zone_for(
        &self,
        state: &State,
        card_id: &uuid::Uuid,
    ) -> anyhow::Result<bool> {
        if !self.is_in_play() {
            return Ok(false);
        }

        let should_override = state
            .continuous_effects
            .iter()
            .filter(|e| match e {
                ContinuousEffect::OverrideValidPlayZone {
                    affected_zones,
                    affected_cards,
                    ..
                } => affected_zones.contains(self) && affected_cards.matches(card_id, state),
                _ => false,
            })
            .count()
            > 0;
        if should_override {
            return Ok(true);
        }

        match self {
            Zone::Realm(_) => {
                let site_in_zone = self.get_site(state);
                if let Some(site) = site_in_zone {
                    return site.is_valid_play_zone_for(state, card_id);
                }

                // If there's no site in the zone, only cards with Voidwalk can be played there.
                let card = state.get_card(card_id);
                match card.get_card_type() {
                    CardType::Site => {
                        let player_id = card.get_controller_id(state);
                        let has_played_site = !CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(&player_id)
                            .all(state)
                            .is_empty();
                        if !has_played_site {
                            let avatar_id = state.get_player_avatar_id(&player_id)?;
                            let avatar = state.get_card(&avatar_id);
                            return Ok(avatar.get_zone() == self);
                        }

                        let empty_adjacent_zones: Vec<Zone> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(&player_id)
                            .not_named(Rubble::NAME)
                            .all(state)
                            .into_iter()
                            .map(|cid| state.get_card(&cid).get_zone())
                            .flat_map(|z| z.get_adjacent())
                            .filter(|z| z.get_site(state).is_none())
                            .collect();

                        Ok(empty_adjacent_zones.contains(self))

                        // let occupied_squares: Vec<&Zone> = CardQuery::new()
                        //     .sites()
                        //     .in_play()
                        //     .not_named(&Rubble::NAME)
                        //     .controlled_by(&player_id)
                        //     .all(state)
                        //     .into_iter()
                        //     .map(|cid| state.get_card(&cid).get_zone())
                        //     .collect();
                        //
                        // Ok(occupied_squares
                        //     .iter()
                        //     .flat_map(|c| get_adjacent_zones(c))
                        //     .filter(|c| !occupied_squares.contains(&c))
                        //     .filter(|c| !sites.contains(&c))
                        //     .find(|c| c == self)
                        //     .is_some())
                    }
                    _ => Ok(card.has_ability(state, &Ability::Voidwalk)),
                }
            }
            Zone::Intersection(sqs) => {
                let card = state.get_card(card_id);
                let player_id = card.get_controller_id(state);
                match card.get_card_type() {
                    CardType::Minion => {
                        if !card.is_oversized(state) {
                            return Ok(false);
                        }

                        let site_squares: Vec<u8> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(&player_id)
                            .all(state)
                            .into_iter()
                            .filter_map(|cid| state.get_card(&cid).get_zone().get_square())
                            .collect();
                        Ok(sqs.iter().any(|sq| site_squares.contains(sq)))
                    }
                    CardType::Aura => {
                        let site_squares: Vec<u8> = CardQuery::new()
                            .sites()
                            .in_play()
                            .controlled_by(&player_id)
                            .all(state)
                            .into_iter()
                            .filter_map(|cid| state.get_card(&cid).get_zone().get_square())
                            .collect();
                        Ok(sqs.iter().any(|sq| site_squares.contains(sq)))
                    }
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
        (1..=20).map(Zone::Realm).collect()
    }

    pub fn all_board() -> Vec<Zone> {
        let mut zones = Self::all_realm();
        zones.extend(Self::all_intersections());
        zones
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

    pub fn get_site<'a>(&self, state: &'a State) -> Option<&'a dyn Site> {
        CardQuery::new()
            .sites()
            .in_zone(self)
            .first(state)
            .and_then(|site_id| state.get_card(&site_id).get_site())
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
                    if let Zone::Intersection(locs) = &intersection
                        && locs == &new_squares
                    {
                        return Some(intersection);
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
    pub controller_id: PlayerId,
    pub tapped: bool,
    pub edition: Edition,
    pub zone: Zone,
    pub region: Region,
    pub card_type: CardType,
    pub abilities: Vec<Ability>,
    pub damage_taken: u16,
    pub bearer: Option<uuid::Uuid>,
    pub rarity: Rarity,
    pub power: u16,
    pub has_attachments: bool,
    pub image_path: String,
    pub is_token: bool,
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
pub enum CostAction {
    Tap,
    Discard,
    Sacrifice,
    Surface,
}

#[derive(Debug, Clone)]
pub struct AdditionalCost {
    pub card: CardQuery,
    pub action: CostAction,
}

impl From<uuid::Uuid> for CardQuery {
    fn from(val: uuid::Uuid) -> Self {
        CardQuery::from_id(val)
    }
}

impl From<&uuid::Uuid> for CardQuery {
    fn from(val: &uuid::Uuid) -> Self {
        CardQuery::from_id(*val)
    }
}

impl AdditionalCost {
    pub fn tap(card: impl Into<CardQuery>) -> Self {
        Self {
            card: card.into(),
            action: CostAction::Tap,
        }
    }

    pub fn discard(card: impl Into<CardQuery>) -> Self {
        Self {
            card: card.into(),
            action: CostAction::Discard,
        }
    }

    pub fn sacrifice(card: impl Into<CardQuery>) -> Self {
        Self {
            card: card.into(),
            action: CostAction::Sacrifice,
        }
    }

    pub fn surface(card: impl Into<CardQuery>) -> Self {
        Self {
            card: card.into(),
            action: CostAction::Surface,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CostType {
    // VariableManaCost is used for cards that have a variable cost (represented by X on the card).
    // When paying for a cost with VariableManaCost, the player will be prompted to choose how much
    // mana to pay, up to the amount of mana they have available. The chosen amount will then be
    // paid and returned as a ManaCost in the paid cost.
    VariableManaCost,
    ManaCost(u8),
    Thresholds(Thresholds),
    Additional(Vec<AdditionalCost>),
}

impl Default for CostType {
    fn default() -> Self {
        CostType::ManaCost(0)
    }
}

impl CostType {
    async fn pay(&self, state: &mut State, player_id: &PlayerId) -> anyhow::Result<CostType> {
        match self {
            CostType::Thresholds(_) => Ok(self.clone()),
            CostType::VariableManaCost => {
                let resources = state.get_player_resources(player_id)?;
                let max_mana = resources.mana;
                if max_mana == 0 {
                    anyhow::bail!("No mana available to pay variable mana cost");
                }
                let mana_to_pay =
                    pick_amount(player_id, 1, max_mana, state, "Choose how much mana to pay")
                        .await?;
                let mana = state.get_player_mana_mut(player_id);
                *mana = mana.saturating_sub(mana_to_pay);
                Ok(CostType::ManaCost(mana_to_pay))
            }
            CostType::ManaCost(mc) => {
                let mana = state.get_player_mana_mut(player_id);
                *mana = mana.saturating_sub(*mc);
                Ok(self.clone())
            }
            CostType::Additional(additional_costs) => {
                for ac in additional_costs {
                    let ac = ac.clone();
                    let mut query = ac.card;
                    match ac.action {
                        CostAction::Tap => query = query.untapped(),
                        CostAction::Discard => query = query.in_zone(&Zone::Hand),
                        CostAction::Sacrifice => query = query.in_zones(&Zone::all_realm()),
                        CostAction::Surface => {
                            query = query.in_regions(vec![Region::Underwater, Region::Underground])
                        }
                    }

                    let options = query.all(state);
                    let effect = match options.len() {
                        0 => unreachable!(),
                        1 => {
                            let card_id = options
                                .first()
                                .expect("options to have exactly one element");
                            match ac.action {
                                CostAction::Tap => Effect::TapCard { card_id: *card_id },
                                CostAction::Discard => Effect::DiscardCard {
                                    card_id: *card_id,
                                    player_id: *player_id,
                                },
                                CostAction::Sacrifice => Effect::BuryCard { card_id: *card_id },
                                CostAction::Surface => Effect::SetCardRegion {
                                    card_id: *card_id,
                                    region: Region::Surface,
                                    tap: false,
                                },
                            }
                        }
                        _ => {
                            let card_id = pick_card(
                                player_id,
                                &options,
                                state,
                                "Choose a card to tap for additional cost",
                            )
                            .await?;
                            match ac.action {
                                CostAction::Tap => Effect::TapCard { card_id },
                                CostAction::Discard => Effect::DiscardCard {
                                    card_id,
                                    player_id: *player_id,
                                },
                                CostAction::Sacrifice => Effect::BuryCard { card_id },
                                CostAction::Surface => Effect::SetCardRegion {
                                    card_id,
                                    region: Region::Surface,
                                    tap: true,
                                },
                            }
                        }
                    };

                    effect.apply(state).await?;
                    state
                        .effect_log
                        .push(LoggedEffect::new(Arc::new(effect), state.turns));
                    crate::game::force_sync(player_id, state).await?;
                }

                Ok(self.clone())
            }
        }
    }

    pub fn can_afford(
        &self,
        state: &State,
        player_id: impl AsRef<PlayerId>,
    ) -> anyhow::Result<bool> {
        let resources = state.get_player_resources(player_id.as_ref())?;
        let thresholds = state.get_thresholds_for_player(player_id.as_ref());

        match self {
            CostType::VariableManaCost => Ok(resources.mana > 0),
            CostType::ManaCost(mc) => Ok(resources.mana >= *mc),
            CostType::Thresholds(tc) => Ok(thresholds.fire >= tc.fire
                && thresholds.air >= tc.air
                && thresholds.earth >= tc.earth
                && thresholds.water >= tc.water),
            CostType::Additional(additional_costs) => {
                let mut snapshot = state.snapshot();
                for ac in additional_costs {
                    let ac = ac.clone();
                    let mut query = ac.card;
                    match ac.action {
                        CostAction::Tap => query = query.untapped(),
                        CostAction::Discard => query = query.in_zone(&Zone::Hand),
                        CostAction::Sacrifice => query = query.in_zones(&Zone::all_realm()),
                        CostAction::Surface => {
                            query = query.in_regions(vec![Region::Underwater, Region::Underground])
                        }
                    }

                    let options = query.all(&snapshot);
                    if options.is_empty() {
                        return Ok(false);
                    }

                    let card_id = options
                        .first()
                        .expect("options to have at least one element");
                    match ac.action {
                        CostAction::Tap => snapshot.get_card_mut(card_id).set_tapped(true),
                        CostAction::Discard => {
                            snapshot.get_card_mut(card_id).set_zone(Zone::Cemetery)
                        }
                        CostAction::Sacrifice => {
                            snapshot.get_card_mut(card_id).set_zone(Zone::Cemetery)
                        }
                        CostAction::Surface => {
                            snapshot.get_card_mut(card_id).set_region(Region::Surface)
                        }
                    }
                }

                Ok(true)
            }
        }
    }
}

// Costs represents the different ways a card or ability can be paid for. It is represented as a vec
// of Cost, where each Cost is a different way to pay for the card, and the player can choose which
// one to pay when playing the card.
#[derive(Debug, Clone, Default)]
pub struct Costs(Vec<Cost>);

impl Costs {
    pub const ZERO: Costs = Costs(vec![]);

    pub fn single(cost: Cost) -> Self {
        Self(vec![cost])
    }

    pub fn mana_only(mana: u8) -> Self {
        Self(vec![Cost::mana_only(mana)])
    }

    pub fn basic(mana: u8, thresholds: impl Into<Thresholds>) -> Self {
        Self(vec![Cost::new(mana, thresholds)])
    }

    pub fn threshold_only(thresholds: impl Into<Thresholds>) -> Self {
        Self(vec![Cost::new(0, thresholds)])
    }

    pub fn multi(costs: Vec<Cost>) -> Self {
        Self(costs)
    }

    pub fn mana_cost(&self) -> &Cost {
        if self.0.is_empty() {
            return Cost::ZERO;
        }

        for cost in &self.0 {
            for cost_type in &cost.0 {
                if let CostType::ManaCost(_) = cost_type {
                    return cost;
                }
            }
        }

        Cost::ZERO
    }

    pub fn mana_value(&self) -> u8 {
        if self.0.is_empty() {
            return 0;
        }

        for cost in &self.0 {
            for cost_type in &cost.0 {
                if let CostType::ManaCost(mc) = cost_type {
                    return *mc;
                }
            }
        }

        0
    }

    pub fn thresholds_cost(&self) -> &Thresholds {
        if self.0.is_empty() {
            return &Thresholds::ZERO;
        }

        for cost in &self.0 {
            for cost_type in &cost.0 {
                if let CostType::Thresholds(tc) = cost_type {
                    return tc;
                }
            }
        }

        &Thresholds::ZERO
    }

    pub fn with_alternative(mut self, alternative_cost: Cost) -> Self {
        self.0.push(alternative_cost);
        Self(self.0)
    }

    /// Returns a new `Costs` with every mana component adjusted by `diff`, clamped to 0.
    pub fn with_mana_adjusted(&self, diff: i8) -> Self {
        Self(self.0.iter().map(|c| c.with_mana_adjusted(diff)).collect())
    }

    pub async fn pay(&self, state: &mut State, player_id: &PlayerId) -> anyhow::Result<Cost> {
        match self.0.len() {
            0 => Ok(Cost::ZERO.clone()),
            1 => {
                let cost = self.0.first().expect("costs to have one item");
                Box::pin(cost.pay(state, player_id)).await
            }
            _ => {
                let affordable_costs = self
                    .0
                    .iter()
                    .filter(|c| c.can_afford(state, player_id).unwrap_or(false))
                    .cloned()
                    .collect::<Vec<_>>();
                let cost_labels = affordable_costs
                    .iter()
                    .map(|c| c.get_label())
                    .collect::<Vec<_>>();
                let picked_cost_idx =
                    pick_option(player_id, &cost_labels, state, "Pick a cost to pay", false)
                        .await?;
                Box::pin(affordable_costs[picked_cost_idx].pay(state, player_id)).await
            }
        }
    }

    pub fn can_afford(
        &self,
        state: &State,
        player_id: impl AsRef<PlayerId>,
    ) -> anyhow::Result<bool> {
        if self.0.is_empty() {
            return Ok(true);
        }

        for cost in &self.0 {
            if cost.can_afford(state, player_id.as_ref())? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

// Cost represents the cost to play a card or activate an ability. It is represented as a vec of
// vecs of CostType, where each CostType is a different type of cost (e.g: mana, threshold,
// additional costs) and all of them must be paid together.
#[derive(Debug, Clone, Default)]
pub struct Cost(Vec<CostType>);

impl IntoIterator for Cost {
    type Item = CostType;
    type IntoIter = std::vec::IntoIter<CostType>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Cost {
    pub const ZERO: &'static Cost = &Cost(vec![]);

    pub fn new(mana: u8, thresholds: impl Into<Thresholds>) -> Self {
        let mut basic_cost = vec![];
        if mana > 0 {
            basic_cost.push(CostType::ManaCost(mana));
        }

        let thresholds = thresholds.into();
        if thresholds != Thresholds::ZERO {
            basic_cost.push(CostType::Thresholds(thresholds));
        }

        Self(basic_cost)
    }

    pub fn from_variable_mana(thresholds: impl Into<Thresholds>) -> Self {
        let mut basic_cost = vec![CostType::VariableManaCost];
        let thresholds = thresholds.into();
        if thresholds != Thresholds::ZERO {
            basic_cost.push(CostType::Thresholds(thresholds));
        }

        Self(basic_cost)
    }

    pub fn mana_only(mana: u8) -> Self {
        Self::new(mana, Thresholds::ZERO)
    }

    pub fn thresholds_only(thresholds: impl Into<Thresholds>) -> Self {
        Self::new(0, thresholds)
    }

    pub fn additional_only(additional: AdditionalCost) -> Self {
        Self(vec![CostType::Additional(vec![additional])])
    }

    pub fn with_additional(mut self, additional: AdditionalCost) -> Self {
        for cost_type in &mut self.0 {
            if let CostType::Additional(additional_costs) = cost_type {
                additional_costs.push(additional);
                return Self(self.0);
            }
        }
        self.0.push(CostType::Additional(vec![additional]));
        Self(self.0)
    }

    pub fn get_label(&self) -> String {
        let mut parts = vec![];
        for cost_type in &self.0 {
            match cost_type {
                CostType::VariableManaCost => parts.push("X Mana".to_string()),
                CostType::ManaCost(mc) => parts.push(format!("{} Mana", mc)),
                CostType::Thresholds(tc) => parts.push(format!("Thresholds: {:?}", tc)),
                CostType::Additional(additional) => {
                    for add in additional {
                        match add.action {
                            CostAction::Tap => parts.push("Tap card".to_string()),
                            CostAction::Discard => parts.push("Discard card".to_string()),
                            CostAction::Sacrifice => parts.push("Sacrifice card".to_string()),
                            CostAction::Surface => parts.push("Put card on Surface".to_string()),
                        }
                    }
                }
            }
        }

        parts.join(" + ")
    }

    /// Returns a copy of this `Cost` with every `ManaCost` adjusted by `diff`, clamped to 0.
    pub fn with_mana_adjusted(&self, diff: i8) -> Self {
        Self(
            self.0
                .iter()
                .map(|ct| match ct {
                    CostType::ManaCost(mc) => {
                        CostType::ManaCost(((*mc as i16) + (diff as i16)).max(0) as u8)
                    }
                    other => other.clone(),
                })
                .collect(),
        )
    }

    pub async fn pay(&self, state: &mut State, player_id: &PlayerId) -> anyhow::Result<Cost> {
        let mut paid_cost = Cost(vec![]);
        for cost_type in &self.0 {
            let paid = cost_type.pay(state, player_id).await?;
            paid_cost.0.push(paid);
        }

        Ok(paid_cost)
    }

    pub fn can_afford(
        &self,
        state: &State,
        player_id: impl AsRef<PlayerId>,
    ) -> anyhow::Result<bool> {
        for cost_type in &self.0 {
            if !cost_type.can_afford(state, player_id.as_ref())? {
                return Ok(false);
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
    fn get_description(&self) -> &str {
        ""
    }
    fn get_base(&self) -> &CardBase;
    fn get_base_mut(&mut self) -> &mut CardBase;

    fn get_edition(&self) -> &Edition {
        &self.get_base().edition
    }

    fn is_tapped(&self) -> bool {
        if let Some(sb) = self.get_site_base() {
            return sb.tapped;
        }
        if let Some(ub) = self.get_unit_base() {
            return ub.tapped;
        }
        if let Some(ab) = self.get_artifact_base() {
            return ab.tapped;
        }
        if let Some(aura_b) = self.get_aura_base() {
            return aura_b.tapped;
        }
        false
    }

    fn set_tapped(&mut self, value: bool) {
        if let Some(sb) = self.get_site_base_mut() {
            sb.tapped = value;
            return;
        }
        if let Some(ub) = self.get_unit_base_mut() {
            ub.tapped = value;
            return;
        }
        if let Some(ab) = self.get_artifact_base_mut() {
            ab.tapped = value;
            return;
        }
        if let Some(aura_b) = self.get_aura_base_mut() {
            aura_b.tapped = value;
        }
    }

    fn set_region(&mut self, region: Region) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.region = region;
            return;
        }
        if let Some(ab) = self.get_artifact_base_mut() {
            ab.region = region;
            return;
        }
        if let Some(aura_b) = self.get_aura_base_mut() {
            aura_b.region = region;
        }
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.get_base().id
    }

    fn get_image_path(&self) -> String {
        use unidecode::unidecode;

        let set = self.get_edition().url_name();
        let name_for_url = unidecode(self.get_name())
            .to_string()
            .to_lowercase()
            .replace("'", "")
            .replace(" ", "_")
            .replace("-", "_");
        let mut folder = "cards";
        if self.is_site() {
            folder = "rotated";
        }
        let mut after_card_name = "b";
        if self.get_base().is_token {
            after_card_name = "bt";
        }

        format!(
            "https://d27a44hjr9gen3.cloudfront.net/{}/{}-{}-{}-s.png",
            folder, set, name_for_url, after_card_name
        )
    }

    // When resolving a CardQuery, this method allows the card to override the query. A useful
    // usecase for this method is for example overriding the valid targets of a spell when there's
    // a card in play that affects targeting.
    async fn card_query_override(
        &self,
        _state: &State,
        _query: &CardQuery,
    ) -> anyhow::Result<Option<CardQuery>> {
        Ok(None)
    }

    // When resolving a ZoneQuery, this method allows the card to override the query. A useful
    // usecase for this method is for example overriding the zones that the player can pick from
    // when the there's a card in play that affects zone selection.
    fn zone_query_override(
        &self,
        _state: &State,
        _query: &ZoneQuery,
    ) -> anyhow::Result<Option<ZoneQuery>> {
        Ok(None)
    }

    // Allows any in-play card to restrict the valid targets of a CardQuery being resolved.
    // This is called for all in-play cards before any pick is made (for both randomised and
    // player-chosen targeting). If a card's presence mandates that certain targets must be
    // chosen over others, return Some with the filtered list. Return None to leave unchanged.
    fn restrict_card_query_targets(
        &self,
        _state: &State,
        _query: &CardQuery,
        _targets: &[uuid::Uuid],
    ) -> Option<Vec<uuid::Uuid>> {
        None
    }

    // When resolving an effect, this methods allows a card in play to replace that event with a
    // different set of effects.
    async fn replace_effect(
        &self,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Option<Vec<Effect>>> {
        Ok(None)
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
    fn get_controller_id(&self, state: &State) -> PlayerId {
        let mut controller = self.get_base().controller_id;
        if let Some(artifact) = self.get_artifact()
            && let Some(bearer_id) = artifact.get_bearer().unwrap_or_default()
        {
            let bearer = state.get_card(&bearer_id);
            // Artifacts are controlled by the controller of their bearer.
            controller = bearer.get_controller_id(state);
        }

        for we in &state.continuous_effects {
            if let ContinuousEffect::ControllerOverride {
                controller_id,
                affected_cards,
            } = we
                && affected_cards.matches(self.get_id(), state)
            {
                controller = *controller_id;
            }
        }

        controller
    }

    fn get_bearer_id(&self) -> anyhow::Result<Option<uuid::Uuid>> {
        if let Some(artifact) = self.get_artifact() {
            return artifact.get_bearer();
        }

        Ok(self.get_base().bearer)
    }

    fn set_bearer_id(&mut self, bearer_id: Option<uuid::Uuid>) {
        self.get_base_mut().bearer = bearer_id;
    }

    async fn after_ranged_attack(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    // Returns a list of effects that must be applied when this card is defending against an
    // attack.
    fn on_defend(&self, state: &State, attacker_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        if let Some(power) = self.get_power(state)? {
            return Ok(vec![Effect::TakeDamage {
                card_id: *attacker_id,
                from: *self.get_id(),
                damage: power,
                is_strike: false,
            }]);
        }

        Ok(vec![])
    }

    // Sets custom data for the card. By default, this method returns an error indicating that
    // the operation is not implemented for the specific card type.
    // If a card needs to hold specific data, and you need to modify it, override this method with
    // a method that downcasts the data to the appropriate type and sets it on the card.
    fn set_data(&mut self, _data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "set_data not implemented for {}",
            self.get_name()
        ))
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

                if self.has_ability(state, &Ability::Airborne) {
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

        if self.is_unit() && !self.has_ability(state, &Ability::Voidwalk) {
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
    fn base_take_damage(
        &mut self,
        state: &State,
        from: &uuid::Uuid,
        damage: u16,
    ) -> anyhow::Result<Vec<Effect>> {
        match self.get_card_type() {
            CardType::Minion => {
                // Check LethalTarget before the mutable borrow of unit_base.
                let has_lethal_target = self.get_unit_base().is_some_and(|ub| {
                    ub.abilities.contains(&Ability::LethalTarget)
                        || ub
                            .modifier_counters
                            .iter()
                            .any(|c| c.ability == Ability::LethalTarget)
                }) || state.continuous_effects.iter().any(|ce| match ce {
                    ContinuousEffect::GrantAbility {
                        ability: Ability::LethalTarget,
                        affected_cards,
                    } => affected_cards.matches(self.get_id(), state),
                    _ => false,
                });

                let ub = self
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("unit card has no unit base"))?;
                ub.damage += damage;

                let mut effects = vec![];
                let attacker = state.get_card(from);
                if ub.damage >= self.get_toughness(state).unwrap_or(0)
                    || attacker.has_ability(state, &Ability::Lethal)
                    || has_lethal_target
                {
                    effects.push(Effect::KillMinion {
                        card_id: *self.get_id(),
                        killer_id: *from,
                    });
                }

                // Lifesteal: if the defender is a unit, heal the attacker's controller.
                let defender = state.get_card(self.get_id());
                if attacker.has_ability(state, &Ability::Lifesteal) && defender.is_unit() {
                    let controller_id = attacker.get_controller_id(state);
                    if let Ok(avatar_id) = state.get_player_avatar_id(&controller_id) {
                        let heal = attacker.get_power(state)?.unwrap_or(0);
                        if heal > 0 {
                            effects.push(Effect::Heal {
                                card_id: avatar_id,
                                amount: heal,
                            });
                        }
                    }
                }

                Ok(effects)
            }
            CardType::Avatar => {
                let ab = self
                    .get_avatar_base()
                    .ok_or(anyhow::anyhow!("avatar card has no avatar base"))?;
                if ab.deaths_door && !ab.can_die {
                    return Ok(vec![]);
                }

                if ab.deaths_door && ab.can_die {
                    return Ok(vec![Effect::PlayerLost {
                        player_id: self.get_controller_id(state),
                    }]);
                }

                let ub = self
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("unit card has no unit base"))?;
                ub.damage += damage;

                if ub.damage >= self.get_toughness(state).unwrap_or(0) {
                    let ab = self
                        .get_avatar_base_mut()
                        .ok_or(anyhow::anyhow!("avatar card has no avatar base"))?;
                    ab.deaths_door = true;
                }

                let attacker = state.get_card(from);
                let mut effects = vec![];
                // Lifesteal: if the defender is a unit, heal the attacker's controller.
                let defender = state.get_card(self.get_id());
                if attacker.has_ability(state, &Ability::Lifesteal) && defender.is_unit() {
                    let controller_id = attacker.get_controller_id(state);
                    if let Ok(avatar_id) = state.get_player_avatar_id(&controller_id) {
                        let heal = attacker.get_power(state)?.unwrap_or(0);
                        if heal > 0 {
                            effects.push(Effect::Heal {
                                card_id: avatar_id,
                                amount: heal,
                            });
                        }
                    }
                }

                Ok(effects)
            }
            CardType::Site => {
                let avatar_id = state.get_player_avatar_id(&self.get_controller_id(state))?;
                Ok(vec![Effect::TakeDamage {
                    card_id: avatar_id,
                    from: *from,
                    damage,
                    is_strike: false,
                }])
            }
            _ => Ok(vec![]),
        }
    }

    // Base on-summon behaviour for site cards. This method MUST NOT BE OVERRIDEN by specific card
    // implementations. Instead, specific card types should override `on_summon`, and can use
    // base_site_on_summon to get the default behaviour.
    fn base_site_on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let site_base = self
            .get_site()
            .ok_or(anyhow::anyhow!("site card has no site base"))?;
        Ok(vec![Effect::AddMana {
            player_id: *self.get_owner_id(),
            mana: site_base.provided_mana(state)?,
        }])
    }

    fn default_get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        let controller_id = self.get_controller_id(state);
        Ok(Zone::all_board()
            .into_iter()
            .filter(|z| {
                let costs = state
                    .get_effective_costs(self.get_id(), Some(z))
                    .unwrap_or_default();
                let can_afford = costs.can_afford(state, controller_id).unwrap_or_default();
                if !can_afford {
                    return false;
                }

                z.is_valid_play_zone_for(state, self.get_id())
                    .unwrap_or_default()
            })
            .collect::<Vec<Zone>>())
    }

    // Retuns the region the card is currently on. If the card is not in a zone with a site, it is
    // in the void.
    fn get_region(&self, state: &State) -> &Region {
        if self.get_zone().get_site(state).is_none() {
            return &Region::Void;
        }

        if let Some(ub) = self.get_unit_base() {
            return &ub.region;
        }
        if let Some(ab) = self.get_artifact_base() {
            return &ab.region;
        }
        if let Some(aura_b) = self.get_aura_base() {
            return &aura_b.region;
        }
        &Region::Void
    }

    fn is_flooded_site(&self, state: &State) -> bool {
        if !self.is_site() {
            return false;
        }

        self.get_site()
            .and_then(|site| site.is_flooded(state).ok())
            .unwrap_or(false)
    }

    // Returns the amount of damage taken by the card. Defaults to 0 for non-unit cards.
    fn get_damage_taken(&self) -> anyhow::Result<u16> {
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
        for modif in self.get_abilities(state)? {
            if let Ability::Ranged(_) = modif {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Returns whether the card has the given modifier.
    fn has_ability(&self, state: &State, ability: &Ability) -> bool {
        let has_temporary = state.temporary_effects.iter().any(|te| match te {
            TemporaryEffect::GrantAbility {
                affected_cards,
                ability: granted_ability,
                ..
            } => {
                if ability != granted_ability {
                    return false;
                }

                if !affected_cards.matches(self.get_id(), state) {
                    return false;
                }

                true
            }
            _ => false,
        });

        if has_temporary {
            return true;
        }

        if self
            .get_unit_base()
            .unwrap_or(&UnitBase::default())
            .abilities
            .contains(ability)
        {
            return true;
        }

        if self
            .get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifier_counters
            .iter()
            .any(|c| &c.ability == ability)
        {
            return true;
        }

        // Also check abilities granted via continuous effects.
        state.continuous_effects.iter().any(|ce| match ce {
            ContinuousEffect::GrantAbility {
                ability: granted_ability,
                affected_cards,
            } if granted_ability == ability => affected_cards.matches(self.get_id(), state),
            _ => false,
        })
    }

    /// Returns true if this is an oversized unit (occupies a 2×2 intersection).
    fn is_oversized(&self, state: &State) -> bool {
        self.has_ability(state, &Ability::Oversized)
    }

    /// Returns true if this card physically occupies `zone`.
    ///
    /// For normal cards this is equivalent to `self.get_zone() == zone`.
    /// For oversized units at a `Zone::Intersection`, the unit also occupies
    /// each of the four constituent `Zone::Realm` sub-zones.
    fn occupies_zone(&self, state: &State, zone: &Zone) -> bool {
        if self.get_zone() == zone {
            return true;
        }
        if self.is_oversized(state)
            && let Zone::Intersection(sub_zones) = self.get_zone()
            && let Zone::Realm(sq) = zone
        {
            return sub_zones.contains(sq);
        }
        false
    }

    // Returns the elements associated to this card.
    fn get_elements(&self, state: &State) -> anyhow::Result<Vec<Element>> {
        let thresholds = if self.is_site() {
            self.get_site_base()
                .ok_or(anyhow::anyhow!("site card has no site base"))?
                .provided_thresholds
                .clone()
        } else {
            self.get_costs(state)?.thresholds_cost().clone()
        };

        let mut elements = Vec::new();
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

    fn zones_in_range(&self, state: &State) -> Vec<Zone> {
        self.get_zones_within_steps(state, self.get_steps_per_movement(state).unwrap_or(0))
    }

    // Returns all valid move paths from the card's current zone to the given zone. The paths
    // include the starting, ending and all intermediate zones.
    fn get_valid_move_paths(&self, state: &State, to: &Zone) -> anyhow::Result<Vec<Vec<Zone>>> {
        let from = self.get_zone().clone();
        let valid_zones = self.get_valid_move_zones(state)?;
        if !valid_zones.contains(to) {
            return Ok(vec![]);
        }

        let max_steps = self.get_steps_per_movement(state)?;
        let is_traversable = |current: &Zone, next: &Zone| {
            self.get_zones_within_steps_of(state, 1, current)
                .contains(next)
        };

        let mut paths = Vec::new();
        let mut queue: Vec<(Vec<Zone>, Zone)> = vec![(vec![from.clone()], from.clone())];
        while let Some((path, current)) = queue.pop() {
            if &current == to {
                if path.len() - 1 <= max_steps.into() {
                    paths.push(path.clone());
                }
                continue;
            }
            if path.len() > max_steps.into() {
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
        if self.has_ability(state, &Ability::Immobile) {
            return Ok(0);
        }

        let extra_steps: u8 = self
            .get_abilities(state)?
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

                if self.has_ability(state, &Ability::Voidwalk) {
                    return true;
                }

                // Oversized units may only move to intersection zones where all 4 sub-zones have sites.
                if self.is_oversized(state) {
                    return match z {
                        Zone::Intersection(sqs) => sqs
                            .iter()
                            .all(|sq| Zone::Realm(*sq).get_site(state).is_some()),
                        _ => false,
                    };
                }

                z.get_site(state).is_some_and(|c| {
                    c.can_be_entered_by(
                        self.get_id(),
                        self.get_zone(),
                        self.get_region(state),
                        state,
                    )
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
    fn get_valid_attack_targets_from_zone(
        &self,
        state: &State,
        ranged: bool,
        zone: &Zone,
    ) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| c.get_controller_id(state) != self.get_controller_id(state))
            .filter(|c| c.is_unit() || c.is_site())
            .filter(|c| c.can_be_targetted_by_player(state, &self.get_controller_id(state)))
            .filter(|c| !c.has_ability(state, &Ability::Unattackable))
            .filter(|c| {
                let same_region = c.get_region(state) == self.get_region(state);
                let ranged_on_airborne = ranged
                    && self.get_region(state) == &Region::Surface
                    && c.has_ability(state, &Ability::Airborne);
                let airborne_on_surface = self.has_ability(state, &Ability::Airborne)
                    && c.get_region(state) == &Region::Surface;
                same_region || ranged_on_airborne || airborne_on_surface
            })
            .filter(|_| {
                let attacker_is_airborne = self.has_ability(state, &Ability::Airborne);
                if !attacker_is_airborne {
                    return zone.is_adjacent(self.get_zone());
                }

                zone.is_nearby(self.get_zone())
            })
            .map(|c| *c.get_id())
            .collect()
    }

    // Returns the valid attack targets for this card.
    fn get_valid_attack_targets(&self, state: &State, ranged: bool) -> Vec<uuid::Uuid> {
        self.get_valid_attack_targets_from_zone(state, ranged, self.get_zone())
    }

    // Returns the toughness of the card. Returns None for non-unit cards.
    fn get_toughness(&self, state: &State) -> Option<u16> {
        match self.get_unit_base() {
            Some(base) => {
                let mut toughness = base.toughness;
                for counter in &base.power_counters {
                    toughness = toughness.saturating_add_signed(counter.toughness);
                }

                let counters: i16 = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter(|c| !c.is_flooded_site(state))
                    .map(|c| c.area_modifiers(state))
                    .filter_map(|mods| mods.grants_counters.get(self.get_id()).cloned())
                    .flatten()
                    .map(|counter| counter.toughness)
                    .sum();
                toughness = toughness.saturating_add_signed(counters);

                Some(toughness)
            }
            None => None,
        }
    }

    // Returns all modifiers currently applied to the card.
    fn get_abilities(&self, state: &State) -> anyhow::Result<Vec<Ability>> {
        Ok(self.base_get_abilities(state))
    }

    fn base_get_abilities(&self, state: &State) -> Vec<Ability> {
        match self.get_unit_base() {
            Some(base) => {
                let mut modifiers = base.abilities.clone();
                for counter in &base.modifier_counters {
                    modifiers.push(counter.ability.clone());
                }

                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    if card.is_flooded_site(state) {
                        continue;
                    }
                    let mods = card.area_modifiers(state);
                    if let Some(mods) = mods.grants_abilities.get(self.get_id()) {
                        modifiers.extend(mods.clone());
                    }
                }

                if let Some(bearer_id) = self.get_bearer_id().ok().flatten() {
                    let bearer = state.get_card(&bearer_id);
                    for ability in [
                        Ability::Airborne,
                        Ability::Burrowing,
                        Ability::Submerge,
                        Ability::Voidwalk,
                    ] {
                        if bearer.has_ability(state, &ability) {
                            modifiers.push(ability);
                        }
                    }
                }

                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    if card.is_flooded_site(state) {
                        continue;
                    }
                    let mods = card.area_modifiers(state);
                    if let Some(mods) = mods.removes_abilities.get(self.get_id()) {
                        for modif in mods {
                            modifiers.retain(|m| m != modif);
                        }
                    }
                }

                for ce in &state.continuous_effects {
                    match ce {
                        ContinuousEffect::GrantAbility {
                            ability,
                            affected_cards,
                        } if affected_cards.matches(self.get_id(), state) => {
                            modifiers.push(ability.clone())
                        }
                        _ => {}
                    }
                }

                modifiers
            }
            None => vec![],
        }
    }

    fn has_attachments(&self, state: &State) -> anyhow::Result<bool> {
        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            if card.get_bearer_id()? == Some(*self.get_id()) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn base_get_power(&self, state: &State) -> Option<u16> {
        match self.get_unit_base() {
            Some(base) => {
                let mut power = base.power;
                for counter in &base.power_counters {
                    power = power.saturating_add_signed(counter.power);
                }

                for we in &state.continuous_effects {
                    if let ContinuousEffect::ModifyPower {
                        power_diff,
                        affected_cards,
                    } = we
                        && affected_cards.matches(self.get_id(), state)
                    {
                        power = power.saturating_add_signed(*power_diff);
                    }
                }

                let power_counters: i16 = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter(|c| !c.is_flooded_site(state))
                    .map(|c| c.area_modifiers(state))
                    .filter_map(|mods| mods.grants_counters.get(self.get_id()).cloned())
                    .flatten()
                    .map(|counter| counter.power)
                    .sum();
                power = power.saturating_add_signed(power_counters);

                Some(power)
            }
            None => None,
        }
    }

    fn get_power(&self, state: &State) -> anyhow::Result<Option<u16>> {
        Ok(self.base_get_power(state))
    }

    fn get_costs(&self, _state: &State) -> anyhow::Result<&Costs> {
        Ok(&self.get_base().costs)
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

    // Upcasts a card to a resource provider trait object if it implements it, None otherwise.
    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
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

    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
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

    fn deathrite(&self, state: &State, from: &Zone) -> Vec<Effect> {
        if self.is_site() && from.is_in_play() {
            return vec![
                Effect::SummonToken {
                    player_id: self.get_controller_id(state),
                    token_type: TokenType::Rubble,
                    zone: from.clone(),
                },
                Effect::ConsumeMana {
                    player_id: self.get_controller_id(state),
                    mana: 0,
                },
            ];
        }

        vec![]
    }

    fn can_be_targetted_by_player(&self, state: &State, player_id: &PlayerId) -> bool {
        if self.has_ability(state, &Ability::Stealth) && &self.get_controller_id(state) != player_id
        {
            return false;
        }

        true
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

    fn can_cast_spell_with_id(&self, state: &State, spell_id: &uuid::Uuid) -> anyhow::Result<bool> {
        self.can_cast(state, state.get_card(spell_id))
    }

    fn can_cast(&self, state: &State, spell: &dyn Card) -> anyhow::Result<bool> {
        if !self.get_zone().is_in_play() {
            return Ok(false);
        }

        if self.get_controller_id(state) != spell.get_controller_id(state) {
            return Ok(false);
        }

        if self.is_avatar() {
            return Ok(true);
        }

        if self.has_ability(state, &Ability::Spellcaster(None)) {
            return Ok(true);
        }

        let elements = spell.get_elements(state)?;
        for element in elements {
            if self.has_ability(state, &Ability::Spellcaster(Some(element))) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn on_move(&self, _state: &State, _path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    async fn on_visit_zone(
        &self,
        _state: &State,
        _from: &Zone,
        _to: &Zone,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn on_take_damage(
        &mut self,
        state: &State,
        from: &uuid::Uuid,
        damage: u16,
    ) -> anyhow::Result<Vec<Effect>> {
        self.base_take_damage(state, from, damage)
    }

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    async fn on_turn_end(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn remove_modifier(&mut self, modifier: &Ability) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.abilities.retain(|a| a != modifier);
            // Also remove any ability counters with this ability type.
            ub.modifier_counters.retain(|c| &c.ability != modifier);
        }
    }

    fn add_modifier(&mut self, modifier: Ability) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.abilities.push(modifier);
        }
    }

    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if self.is_site() {
            return self.base_site_on_summon(state);
        }

        Ok(vec![])
    }

    async fn on_cast(
        &mut self,
        _state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    /// Called on the Spellcaster unit after it successfully casts a spell.
    /// `spell_id` is the UUID of the spell card just cast.
    async fn on_cast_spell(
        &self,
        _state: &State,
        _spell_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    async fn play_mechanic(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let card_id = self.get_id();
        match self.get_card_type() {
            CardType::Minion => {
                let zones = self.get_valid_play_zones(state)?;
                let prompt = "Pick a zone to play the card";
                let zone = pick_zone(controller_id, &zones, state, false, prompt).await?;
                Ok(vec![Effect::PlayCard {
                    player_id: controller_id,
                    card_id: *self.get_id(),
                    zone: zone.clone().into(),
                }])
            }
            CardType::Artifact => {
                let units = self
                    .get_artifact()
                    .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                    .get_valid_attach_targets(state);
                let needs_bearer = state
                    .get_card(card_id)
                    .get_artifact()
                    .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                    .needs_bearer(state)?;
                match needs_bearer {
                    true => {
                        let picked_card_id = pick_card(
                            controller_id,
                            &units,
                            state,
                            format!("Pick a unit to attach {} to", self.get_name()).as_str(),
                        )
                        .await?;
                        let picked_card = state.get_card(&picked_card_id);
                        Ok(vec![
                            Effect::SetBearer {
                                card_id: *card_id,
                                bearer_id: Some(picked_card_id),
                            },
                            Effect::PlayCard {
                                player_id: controller_id,
                                card_id: *card_id,
                                zone: picked_card.get_zone().clone().into(),
                            },
                        ])
                    }
                    false => {
                        let picked_zone = pick_zone(
                            controller_id,
                            &self.get_valid_play_zones(state)?,
                            state,
                            false,
                            "Pick a zone to play the artifact",
                        )
                        .await?;
                        Ok(vec![Effect::PlayCard {
                            player_id: controller_id,
                            card_id: *card_id,
                            zone: picked_zone.clone().into(),
                        }])
                    }
                }
            }
            _ => Ok(vec![]),
        }
    }

    fn on_region_change(
        &self,
        _state: &State,
        _from: &Region,
        _to: &Region,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn base_avatar_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        let mut activated_abilities: Vec<Box<dyn ActivatedAbility>> =
            self.base_unit_activated_abilities(state)?;
        activated_abilities.push(Box::new(AvatarAction::DrawSite));
        if state
            .cards
            .iter()
            .filter(|c| c.get_controller_id(state) == self.get_controller_id(state))
            .filter(|c| c.is_site())
            .filter(|c| matches!(c.get_zone(), Zone::Hand))
            .count()
            > 0
        {
            activated_abilities.push(Box::new(AvatarAction::PlaySite));
        }

        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            let mods = card.area_modifiers(state);
            if card.is_flooded_site(state) {
                continue;
            }
            if let Some(mods) = mods.grants_activated_abilities.get(self.get_id()) {
                activated_abilities.extend(mods.clone());
            }
        }

        Ok(activated_abilities)
    }

    fn base_unit_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        let mut activated_abilities: Vec<Box<dyn ActivatedAbility>> =
            vec![Box::new(UnitAction::Attack), Box::new(UnitAction::Move)];
        if self.is_ranged(state)? {
            activated_abilities.push(Box::new(UnitAction::RangedAttack));
        }

        if let Some(site) = self.get_zone().get_site(state) {
            if self.has_ability(state, &Ability::Burrowing) && site.is_land_site(state)? {
                if self.get_region(state) == &Region::Surface {
                    activated_abilities.push(Box::new(UnitAction::Burrow));
                } else {
                    activated_abilities.push(Box::new(UnitAction::Surface));
                }
            }

            if self.has_ability(state, &Ability::Submerge) && site.is_water_site(state)? {
                if self.get_region(state) == &Region::Surface {
                    activated_abilities.push(Box::new(UnitAction::Submerge));
                } else {
                    activated_abilities.push(Box::new(UnitAction::Surface));
                }
            }
        }

        for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
            let mods = card.area_modifiers(state);
            if card.is_flooded_site(state) {
                continue;
            }
            if let Some(mods) = mods.grants_activated_abilities.get(self.get_id()) {
                activated_abilities.extend(mods.clone());
            }
        }

        let unborne_artifacts: Vec<(uuid::Uuid, String)> = CardQuery::new()
            .artifacts()
            .in_zone(self.get_zone())
            .in_region(self.get_region(state))
            .iter(state)
            .filter_map(|c| c.get_artifact())
            .filter(|c| c.get_bearer().unwrap_or_default().is_none())
            .map(|c| (*c.get_id(), c.get_name().to_string()))
            .collect();
        for (artifact_id, artifact_name) in unborne_artifacts {
            activated_abilities.push(Box::new(UnitAction::PickUpArtifact {
                artifact_id,
                artifact_name,
            }));
        }

        Ok(activated_abilities)
    }

    // Returns the available actions for this card, given the current game state.
    fn get_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.has_ability(state, &Ability::Disabled) {
            return Ok(vec![]);
        }

        if state.permanently_disabled_abilities.contains(self.get_id()) {
            return Ok(vec![]);
        }

        if self.is_site() && self.is_flooded_site(state) {
            return Ok(vec![]);
        }

        if self.is_avatar() {
            let mut abilities = self.base_avatar_activated_abilities(state)?;
            abilities.extend(self.get_additional_activated_abilities(state)?);
            Ok(abilities)
        } else if self.is_unit() {
            let mut abilities = self.base_unit_activated_abilities(state)?;
            abilities.extend(self.get_additional_activated_abilities(state)?);
            Ok(abilities)
        } else {
            let abilities = self.get_additional_activated_abilities(state)?;
            Ok(abilities)
        }
    }

    // Returns a list of additional activated abilities for this card. The base abilities, like
    // attack and move for units, or draw site and play site for avatars, should not be returned in
    // this function, as they are automatically included in the activated abilities of the card
    // based on its type and current state.
    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![])
    }

    // Returns the modifiers that this card provides to other cards in the game.
    fn area_modifiers(&self, _state: &State) -> AreaModifiers {
        AreaModifiers::default()
    }

    fn area_effects(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![])
    }
}

#[derive(Debug, Default, Clone)]
pub struct AreaModifiers {
    pub grants_abilities: HashMap<uuid::Uuid, Vec<Ability>>,
    pub removes_abilities: HashMap<uuid::Uuid, Vec<Ability>>,
    pub grants_activated_abilities: HashMap<uuid::Uuid, Vec<Box<dyn ActivatedAbility>>>,
    pub grants_counters: HashMap<uuid::Uuid, Vec<Counter>>,
}

#[derive(Debug, PartialEq, Clone, EnumIter)]
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
    Merfolk,
    Troll,
    Dwarf,
    Automaton,
    Gnome,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SiteType {
    Desert,
    Tower,
    Earth,
    Village,
    River,
}

#[derive(Debug, Clone, Default)]
pub struct SiteBase {
    pub provided_mana: u8,
    pub provided_thresholds: Thresholds,
    pub types: Vec<SiteType>,
    pub tapped: bool,
}

pub trait ResourceProvider: Card {
    fn provided_mana(&self, state: &State) -> anyhow::Result<u8> {
        if self.get_card_type() != CardType::Site {
            return Ok(0);
        }

        let mut mana = self
            .get_site_base()
            .ok_or(anyhow::anyhow!("site card has no base"))?
            .provided_mana;

        state
            .continuous_effects
            .iter()
            .filter(|ce| match ce {
                ContinuousEffect::ModifyProvidedMana { affected_cards, .. } => {
                    affected_cards.matches(self.get_id(), state)
                }
                _ => false,
            })
            .for_each(|ce| {
                if let ContinuousEffect::ModifyProvidedMana { mana_diff, .. } = ce {
                    mana = mana.saturating_add_signed(*mana_diff);
                }
            });

        Ok(mana)
    }

    fn provided_affinity(&self, state: &State) -> anyhow::Result<Thresholds> {
        if self.get_card_type() != CardType::Site {
            return Ok(Thresholds::ZERO);
        }

        match self.get_card_type() {
            CardType::Site => {
                let site_base = self
                    .get_site_base()
                    .ok_or(anyhow::anyhow!("site card has no base"))?;
                let site = self
                    .get_site()
                    .ok_or(anyhow::anyhow!("site card does not implement site"))?;
                let mut thresholds = site_base.provided_thresholds.clone();
                if site.is_flooded(state)? {
                    thresholds.fire = 0;
                    thresholds.air = 0;
                    thresholds.earth = 0;
                    thresholds.water = std::cmp::max(1, thresholds.water);
                }
                if site.is_droughted(state)? {
                    thresholds.water = 0;
                }

                Ok(thresholds)
            }
            _ => Ok(Thresholds::ZERO),
        }
    }
}

impl<T> ResourceProvider for T where T: Site {}

pub trait Site: Card + ResourceProvider {
    fn is_valid_play_zone_for(&self, state: &State, card_id: &uuid::Uuid) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        if card.is_site() {
            return Ok(false);
        }

        if self.get_controller_id(state) == card.get_controller_id(state) {
            return Ok(true);
        }

        Ok(false)
    }

    fn is_land_site(&self, state: &State) -> anyhow::Result<bool> {
        if let Some(rp) = self.get_resource_provider() {
            return Ok(rp.provided_affinity(state)?.water == 0);
        }

        Ok(false)
    }

    fn is_water_site(&self, state: &State) -> anyhow::Result<bool> {
        if let Some(rp) = self.get_resource_provider() {
            return Ok(rp.provided_affinity(state)?.water != 0);
        }

        Ok(false)
    }

    fn on_card_enter(&self, _state: &State, _card_id: &uuid::Uuid) -> Vec<Effect> {
        vec![]
    }

    fn can_be_entered_by(
        &self,
        _card: &uuid::Uuid,
        _from: &Zone,
        _region: &Region,
        _state: &State,
    ) -> bool {
        true
    }

    fn is_flooded(&self, state: &State) -> anyhow::Result<bool> {
        let temporarily_flooded = state
            .temporary_effects
            .iter()
            .filter(|te| te.affected_cards(state).contains(self.get_id()))
            .find(|te| matches!(te, TemporaryEffect::FloodSites { .. }))
            .is_some();
        if temporarily_flooded {
            return Ok(true);
        }

        Ok(state
            .continuous_effects
            .iter()
            .find(|ce| match ce {
                ContinuousEffect::FloodSites { affected_sites } => {
                    affected_sites.matches(self.get_id(), state)
                }
                _ => false,
            })
            .is_some())
    }

    fn is_droughted(&self, state: &State) -> anyhow::Result<bool> {
        Ok(state.continuous_effects.iter().any(|ce| match ce {
            ContinuousEffect::DroughtSites { affected_sites } => {
                affected_sites.matches(self.get_id(), state)
            }
            _ => false,
        }))
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
    Spellcaster(Option<Element>),
    Charge,
    SummoningSickness,
    TakesNoDamageFromElement(Element),
    Immobile,
    Waterbound,
    Lifesteal,
    FirstStrike,
    Unattackable,
    Uninterceptable,
    /// Unit occupies all four realm locations of a 2×2 intersection simultaneously.
    Oversized,
    /// Any damage dealt to this unit is lethal (kills regardless of amount).
    LethalTarget,
}

#[derive(Debug, Clone)]
pub struct UnitBase {
    pub power: u16,
    pub toughness: u16,
    pub abilities: Vec<Ability>,
    pub damage: u16,
    pub power_counters: Vec<Counter>,
    pub modifier_counters: Vec<AbilityCounter>,
    pub types: Vec<MinionType>,
    pub carried_by: Option<uuid::Uuid>,
    pub tapped: bool,
    pub region: Region,
}

impl Default for UnitBase {
    fn default() -> Self {
        Self {
            power: 0,
            toughness: 0,
            abilities: vec![],
            damage: 0,
            power_counters: vec![],
            modifier_counters: vec![],
            types: vec![],
            carried_by: None,
            tapped: false,
            region: Region::Surface,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
pub enum Rarity {
    #[default]
    Ordinary,
    Exceptional,
    Elite,
    Unique,
}

impl std::fmt::Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rarity::Ordinary => write!(f, "Ordinary"),
            Rarity::Exceptional => write!(f, "Exceptional"),
            Rarity::Elite => write!(f, "Elite"),
            Rarity::Unique => write!(f, "Unique"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ArtifactType {
    Relic,
    Weapon,
    Armor,
    Device,
    Automaton,
    Monument,
    Instrument,
}

#[derive(Debug, Clone)]
pub struct ArtifactBase {
    pub needs_bearer: bool,
    pub types: Vec<ArtifactType>,
    pub tapped: bool,
    pub region: Region,
}

impl Default for ArtifactBase {
    fn default() -> Self {
        Self {
            needs_bearer: false,
            types: vec![],
            tapped: false,
            region: Region::Surface,
        }
    }
}

pub trait Artifact: Card {
    fn needs_bearer(&self, _state: &State) -> anyhow::Result<bool> {
        Ok(self
            .get_artifact_base()
            .ok_or(anyhow::anyhow!("artifact card has no base"))?
            .needs_bearer)
    }

    fn get_valid_attach_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        match self.get_card_type() {
            CardType::Artifact => CardQuery::new()
                .units()
                .controlled_by(&self.get_controller_id(state))
                .all(state),
            _ => vec![],
        }
    }

    fn get_bearer(&self) -> anyhow::Result<Option<uuid::Uuid>> {
        Ok(self.get_base().bearer)
    }
}

#[derive(Debug, Clone)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub controller_id: PlayerId,
    pub zone: Zone,
    pub costs: Costs,
    // In the case of artifacts, bearer is the id of the card that has the artifact equipped. This
    // field can also be used for units to track when another unit is carrying them (e.g. a unit
    // being carried by Beast of Burden).
    pub bearer: Option<uuid::Uuid>,
    pub rarity: Rarity,
    pub edition: Edition,
    pub is_token: bool,
}

impl Default for CardBase {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            owner_id: PlayerId::default(),
            controller_id: PlayerId::default(),
            zone: Zone::default(),
            costs: Costs::default(),
            bearer: None,
            rarity: Rarity::default(),
            edition: Edition::default(),
            is_token: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuraBase {
    pub tapped: bool,
    pub region: Region,
}

impl Default for AuraBase {
    fn default() -> Self {
        Self {
            tapped: false,
            region: Region::Surface,
        }
    }
}

pub trait Aura: Card {
    fn should_dispell(&self, _state: &State) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn base_get_affected_zones(&self, _state: &State) -> Vec<Zone> {
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

    fn get_affected_zones(&self, state: &State) -> Vec<Zone> {
        self.base_get_affected_zones(state)
    }
}

#[derive(Debug, Default, Clone)]
pub struct AvatarBase {
    pub deaths_door: bool,
    pub can_die: bool,
}

pub fn from_name(name: &str, player_id: &PlayerId) -> Box<dyn Card> {
    CARD_CONSTRUCTORS.get(name).unwrap()(*player_id)
}

pub fn from_name_and_zone(name: &str, player_id: &PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}

#[cfg(test)]
mod tests {
    use crate::{
        card::{
            Ability, AdditionalCost, ApprenticeWizard, AridDesert, Card, Cost, OgreGoons,
            RimlandNomads, Zone,
        },
        state::{CardQuery, State},
    };

    #[test]
    fn test_additional_cost_tap() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let cost = Cost::additional_only(AdditionalCost::tap(
            CardQuery::new()
                .untapped()
                .units()
                .in_zone(&Zone::Realm(10)),
        ));
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(!can_afford, "no units in the zone");

        let mut unit = ApprenticeWizard::new(player_id);
        let unit_id = *unit.get_id();
        unit.set_zone(Zone::Realm(10));
        state.cards.push(Box::new(unit));
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(can_afford, "an untapped unit is present in the zone");

        let unit = state.get_card_mut(&unit_id);
        unit.set_tapped(true);
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(!can_afford, "only unit in zone is tapped");
    }

    #[test]
    fn test_additional_cost_two_taps() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let cost = Cost::ZERO
            .clone()
            .with_additional(AdditionalCost::tap(
                CardQuery::new()
                    .untapped()
                    .units()
                    .in_zone(&Zone::Realm(10)),
            ))
            .with_additional(AdditionalCost::tap(
                CardQuery::new()
                    .untapped()
                    .units()
                    .in_zone(&Zone::Realm(10)),
            ));
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(!can_afford, "no units in the zone");

        let mut unit = ApprenticeWizard::new(player_id);
        let unit_id = *unit.get_id();
        unit.set_zone(Zone::Realm(10));
        state.cards.push(Box::new(unit));
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(!can_afford, "only one unit in the zone, two are required");

        let mut unit = ApprenticeWizard::new(player_id);
        unit.set_zone(Zone::Realm(10));
        state.cards.push(Box::new(unit));
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(can_afford, "two untapped units the zone");

        let unit = state.get_card_mut(&unit_id);
        unit.set_tapped(true);
        let can_afford = cost
            .can_afford(&state, player_id)
            .expect("should not error");
        assert!(!can_afford, "only one untapped unit in the zone");
    }

    #[test]
    fn test_get_valid_move_paths_movement_plus_1() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let mut card = RimlandNomads::new(player_id);
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
        let player_id = state.players[0].id;
        let mut card = RimlandNomads::new(player_id);
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
        let player_id = state.players[0].id;
        let mut card = RimlandNomads::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(2));
        state.cards.push(Box::new(card.clone()));

        let paths = card
            .get_valid_move_paths(&state, &Zone::Realm(15))
            .expect("paths to be computed");
        assert_eq!(paths.len(), 3, "Expected 2 paths, got {:?}", paths);
        assert!(paths.contains(&vec![
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(10),
            Zone::Realm(15)
        ]));
        assert!(paths.contains(&vec![
            Zone::Realm(8),
            Zone::Realm(9),
            Zone::Realm(14),
            Zone::Realm(15)
        ]));
        assert!(paths.contains(&vec![
            Zone::Realm(8),
            Zone::Realm(13),
            Zone::Realm(14),
            Zone::Realm(15)
        ]));
    }

    #[test]
    fn test_get_valid_move_zones_basic_movement() {
        let mut state = State::new_mock_state(Zone::all_realm());
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(1));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Movement(1));
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Voidwalk);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
        let player_id = state.players[0].id;
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm(8));
        card.add_modifier(Ability::Airborne);
        card.add_modifier(Ability::Voidwalk);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_move_zones(&state)
            .expect("zones to be computed");
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
    fn test_get_valid_play_zones_site_second_site() {
        let zones_with_sites = vec![Zone::Realm(3)];
        let mut state = State::new_mock_state(zones_with_sites);
        let player_id = state.players[0].id;
        let mut card = AridDesert::new(player_id);
        card.set_zone(Zone::Hand);
        state.cards.push(Box::new(card.clone()));

        let mut zones = card
            .get_valid_play_zones(&state)
            .expect("zones to be computed");
        zones.sort();
        let mut expected = vec![Zone::Realm(8), Zone::Realm(4), Zone::Realm(2)];
        expected.sort();
        assert_eq!(zones, expected);
    }

    #[test]
    fn test_can_afford_cost() {
        let mut state = State::new_mock_state(vec![]);
        let player_id = state.players[0].id;
        *state.get_player_mana_mut(&player_id) = 2;

        let mut card = OgreGoons::new(player_id);
        card.set_zone(Zone::Hand);
        state.cards.push(Box::new(card.clone()));

        let can_afford = card
            .get_costs(&state)
            .unwrap()
            .can_afford(&state, player_id)
            .unwrap();
        assert!(!can_afford);

        *state.get_player_mana_mut(&player_id) = 3;
        let can_afford = card
            .get_costs(&state)
            .unwrap()
            .can_afford(&state, player_id)
            .unwrap();
        assert!(!can_afford);

        let mut arid_desert = AridDesert::new(player_id);
        arid_desert.set_zone(Zone::Realm(3));
        state.cards.push(Box::new(arid_desert));

        // The player now has 3 mana and a fire affinity of 1, so they should be able to afford the
        // Ogre Goons in their hand, which costs 3F.
        let can_afford = card
            .get_costs(&state)
            .unwrap()
            .can_afford(&state, player_id)
            .unwrap();
        assert!(can_afford);
    }
}
