pub mod beta;
pub mod foot_soldier;
pub mod frog;
pub mod rubble;
pub use beta::*;
pub use foot_soldier::*;
pub use frog::*;
pub use rubble::*;

use crate::prelude::*;

#[cfg(test)]
mod card_test;

use crate::{
    effect::{AbilityCounter, Counter, Effect, StatusCounter},
    game::{
        ActivatedAbility, AvatarAction, Element, PlayerId, Thresholds, ThresholdsDiff, UnitAction,
        pick_amount, pick_card, pick_option, pick_zone,
    },
    query::{CardQuery, ZoneQuery},
    state::{AbilityModifier, ContinuousEffect, LoggedEffect, State, TemporaryEffect},
};
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, sync::LazyLock};
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
    pub statuses: Vec<CardStatus>,
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
    T: 'static + Clone + Card,
{
    fn clone_box(&self) -> Box<dyn Card> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Card> {
    fn clone(&self) -> Box<dyn Card> {
        self.clone_box()
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
                        CostAction::Tap => {
                            query = query
                                .untapped()
                                .without_status(&CardStatus::SummoningSickness)
                        }
                        CostAction::Discard => query = query.in_zone(&Zone::Hand),
                        CostAction::Sacrifice => query = query.in_zones(&Zone::all_realm()),
                        CostAction::Surface => {
                            let mut subsurface_zones = Zone::all_in_region(Region::Underwater);
                            subsurface_zones.extend(Zone::all_in_region(Region::Underground));
                            query = query.in_zones(&subsurface_zones)
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
                                CostAction::Tap => Effect::SetTapped {
                                    card_id: *card_id,
                                    tapped: true,
                                },
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
                                CostAction::Tap => Effect::SetTapped {
                                    card_id,
                                    tapped: true,
                                },
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
                    let turn = state.turns;
                    state.effect_log_mut().push(LoggedEffect::new(effect, turn));
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
                let mut snapshot = state.clone();
                for ac in additional_costs {
                    let ac = ac.clone();
                    let mut query = ac.card;
                    match ac.action {
                        CostAction::Tap => {
                            query = query
                                .untapped()
                                .without_status(&CardStatus::SummoningSickness)
                        }
                        CostAction::Discard => query = query.in_zone(&Zone::Hand),
                        CostAction::Sacrifice => query = query.in_zones(&Zone::all_realm()),
                        CostAction::Surface => {
                            let mut subsurface_zones = Zone::all_in_region(Region::Underwater);
                            subsurface_zones.extend(Zone::all_in_region(Region::Underground));
                            query = query.in_zones(&subsurface_zones)
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

    /// Returns a new `Costs` with every mana component adjusted by `diff`, clamped to 0.
    pub fn with_thresholds_adjusted(&self, diff: ThresholdsDiff) -> Self {
        Self(
            self.0
                .iter()
                .map(|c| c.with_thresholds_adjusted(diff.clone()))
                .collect(),
        )
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

    /// Returns a copy of this `Cost` with every `Threshold` adjusted by `diff`, clamped to 0.
    pub fn with_thresholds_adjusted(&self, diff: ThresholdsDiff) -> Self {
        Self(
            self.0
                .iter()
                .map(|ct| match ct {
                    CostType::Thresholds(t) => CostType::Thresholds(t + &diff),
                    other => other.clone(),
                })
                .collect(),
        )
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

    fn is_token(&self) -> bool {
        self.get_base().is_token
    }

    fn is_affordable(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<bool> {
        // A card is playable if affordable at ANY of its valid target zones
        // (accounting for zone-specific cost reductions like Donnybrook Inn).
        let card_id = self.get_id();
        let valid_zones = self.get_valid_play_zones(state, player_id, caster_id)?;
        let affordable = if valid_zones.is_empty() {
            state
                .get_effective_costs(card_id, None, player_id)?
                .can_afford(state, player_id)?
        } else {
            valid_zones.iter().any(|zone| {
                state
                    .get_effective_costs(card_id, Some(zone), player_id)
                    .and_then(|costs| costs.can_afford(state, player_id))
                    .unwrap_or(false)
            })
        };
        if !affordable {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_playable(&self, state: &State, player_id: &PlayerId) -> anyhow::Result<bool> {
        if state
            .temporary_effects()
            .iter()
            .find(|e| match e {
                te @ TemporaryEffect::MakePlayable { by_player, .. } => {
                    te.affected_cards(state).contains(self.get_id()) && by_player == player_id
                }
                _ => false,
            })
            .is_some()
        {
            return Ok(true);
        }

        Ok(self.get_zone() == &Zone::Hand)
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
        let base = self.get_base_mut();
        match &base.zone {
            Zone::Location(sq, _) => {
                let sq = *sq;
                base.zone = Zone::Location(sq, region);
            }
            Zone::Intersection(sqs, _) => {
                let sqs = sqs.clone();
                base.zone = Zone::Intersection(sqs, region);
            }
            _ => {}
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
            .replace("!", "")
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
            ub.ability_counters.retain(|c| &c.id != id);
        }
    }

    fn remove_status_counter(&mut self, id: &uuid::Uuid) {
        self.get_base_mut().status_counters.retain(|c| &c.id != id);
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

        for we in state.active_continuous_effects() {
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

    fn on_attack(&self, _state: &State, _defender_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn strikes_back(&self, state: &State) -> anyhow::Result<bool> {
        if !self.is_unit() || self.has_status(state, &CardStatus::Disabled) {
            return Ok(false);
        }

        Ok(true)
    }

    // Returns a list of effects that must be applied when this card is defending against an
    // attack.
    fn on_defend(&self, _state: &State, _attacker_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    // Sets custom data for the card. By default, this method returns an error indicating that
    // the operation is not implemented for the specific card type.
    // If a card needs to hold specific data, and you need to modify it, override this method with
    // a method that downcasts the data to the appropriate type and sets it on the card.
    fn set_data(
        &mut self,
        _data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "set_data not implemented for {}",
            self.get_name()
        ))
    }

    // Returns the zones that are within the given steps of the specified zone, using this card as
    // the reference for movement capabilities.
    fn get_zones_within_steps_of(&self, state: &State, steps: u8, zone: &Zone) -> Vec<Zone> {
        fn top_bottom_wrapped_neighbours(zone: &Zone) -> Vec<Zone> {
            match zone {
                Zone::Location(id, region) if *id >= 1 && *id <= 5 => {
                    vec![Zone::Location(id + 15, region.clone())]
                }
                Zone::Location(id, region) if *id >= 16 && *id <= 20 => {
                    vec![Zone::Location(id - 15, region.clone())]
                }
                _ => vec![],
            }
        }

        fn left_right_wrapped_neighbours(zone: &Zone) -> Vec<Zone> {
            match zone {
                Zone::Location(id, region) if (*id - 1) % 5 == 0 => {
                    vec![Zone::Location(id + 4, region.clone())]
                }
                Zone::Location(id, region) if *id % 5 == 0 => {
                    vec![Zone::Location(id - 4, region.clone())]
                }
                _ => vec![],
            }
        }

        let wraps_top_and_bottom =
            state
                .active_continuous_effects()
                .into_iter()
                .any(|ce| match ce {
                    ContinuousEffect::ConnectTopBottomEdges { affected_cards } => {
                        affected_cards.matches(self.get_id(), state)
                    }
                    _ => false,
                });

        let wraps_left_and_right =
            state
                .active_continuous_effects()
                .into_iter()
                .any(|ce| match ce {
                    ContinuousEffect::ConnectLeftRightEdges { affected_cards } => {
                        affected_cards.matches(self.get_id(), state)
                    }
                    _ => false,
                });

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
                        if self
                            .can_move_between_zones(state, &current_zone, &nearby)
                            .unwrap_or(false)
                        {
                            to_visit.push((nearby, current_step + 1));
                        }
                    }
                } else {
                    for adjacent in current_zone.get_adjacent() {
                        if self
                            .can_move_between_zones(state, &current_zone, &adjacent)
                            .unwrap_or(false)
                        {
                            to_visit.push((adjacent, current_step + 1));
                        }
                    }

                    if wraps_top_and_bottom {
                        for wrapped in top_bottom_wrapped_neighbours(&current_zone) {
                            if self
                                .can_move_between_zones(state, &current_zone, &wrapped)
                                .unwrap_or(false)
                            {
                                to_visit.push((wrapped, current_step + 1));
                            }
                        }
                    }

                    if wraps_left_and_right {
                        for wrapped in left_right_wrapped_neighbours(&current_zone) {
                            if self
                                .can_move_between_zones(state, &current_zone, &wrapped)
                                .unwrap_or(false)
                            {
                                to_visit.push((wrapped, current_step + 1));
                            }
                        }
                    }

                    if self.has_ability(state, &Ability::Leap) {
                        for landing in leap_destinations(state, &current_zone) {
                            if self
                                .can_move_between_zones(state, &current_zone, &landing)
                                .unwrap_or(false)
                            {
                                to_visit.push((landing, current_step + 1));
                            }
                        }
                    }
                }

                for connected in temporarily_connected_sites(state, self.get_id(), &current_zone) {
                    if self
                        .can_move_between_zones(state, &current_zone, &connected)
                        .unwrap_or(false)
                    {
                        to_visit.push((connected, current_step + 1));
                    }
                }

                for connected in continuously_connected_zones(state, self.get_id()) {
                    if self
                        .can_move_between_zones(state, &current_zone, &connected)
                        .unwrap_or(false)
                    {
                        to_visit.push((connected, current_step + 1));
                    }
                }
            }
        }

        if self.is_unit() && !self.has_ability(state, &Ability::Voidwalk) {
            visited = visited
                .iter()
                .filter(|z| {
                    z.get_site(state).is_some()
                        || is_continuously_connected_zone(state, self.get_id(), z)
                })
                .cloned()
                .collect();
        }

        visited
    }

    // Returns the zones that are within the given steps of this card's current zone.
    fn get_zones_within_steps(&self, state: &State, steps: u8) -> Vec<Zone> {
        self.get_zones_within_steps_of(state, steps, self.get_zone())
    }

    fn can_move_between_zones(
        &self,
        state: &State,
        from: &Zone,
        to: &Zone,
    ) -> anyhow::Result<bool> {
        if !self.is_unit() {
            return Ok(true);
        }

        for ce in state.active_continuous_effects() {
            if let ContinuousEffect::BlockMovementThrough {
                border,
                affected_cards,
            } = ce
                && affected_cards.matches(self.get_id(), state)
                && zones_cross_border(from, to, border)
            {
                return Ok(false);
            }
        }

        if let Some(site) = from.get_site(state)
            && !site.can_be_exited_by(self.get_id(), to, self.get_region(state), state)?
        {
            return Ok(false);
        }

        if let Some(site) = to.get_site(state)
            && !site.can_be_entered_by(self.get_id(), from, self.get_region(state), state)?
        {
            return Ok(false);
        }

        to.can_be_entered_by(state, self.get_id())
    }

    // Retuns the region the card is currently on. If the card is not in a zone with a site, it is
    // in the void.
    fn get_region(&self, _state: &State) -> &Region {
        static VOID: Region = Region::Void;

        let zone = &self.get_base().zone;
        match zone {
            Zone::Location(_, region) => region,
            Zone::Intersection(_, region) => region,
            _ => &VOID,
        }
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
    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>> {
        self.base_get_valid_play_zones(state, player_id, caster_id)
    }

    fn is_ranged(&self, state: &State) -> anyhow::Result<bool> {
        for modif in self.get_abilities(state)? {
            if let Ability::Ranged(_) = modif {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn ranged_range(&self, state: &State) -> anyhow::Result<Option<u8>> {
        let mut range = 0u8;
        for ability in self.get_abilities(state)? {
            if let Ability::Ranged(steps) = ability {
                range = range.saturating_add(steps);
            }
        }

        Ok((range > 0).then_some(range))
    }

    // Returns whether the card has the given modifier.
    fn has_ability(&self, state: &State, ability: &Ability) -> bool {
        self.get_abilities(state)
            .is_ok_and(|abilities| abilities.contains(ability))
    }

    fn get_statuses(&self, state: &State) -> Vec<CardStatus> {
        let mut statuses = self.get_base().statuses.clone();
        statuses.extend(
            self.get_base()
                .status_counters
                .iter()
                .map(|counter| counter.status.clone()),
        );
        statuses.extend(state.granted_statuses_from_continuous_effects(self.get_id()));
        statuses.sort();
        statuses.dedup();
        statuses
    }

    fn has_status(&self, state: &State, status: &CardStatus) -> bool {
        self.get_statuses(state).contains(status)
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
            && let Zone::Intersection(sub_zones, _) = self.get_zone()
            && let Zone::Location(sq, _) = zone
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
    async fn get_valid_move_paths(
        &self,
        state: &State,
        to: &Zone,
    ) -> anyhow::Result<Vec<Vec<Zone>>> {
        let from = self.get_zone().clone();
        let valid_zones = self.get_valid_move_zones(state).await?;
        if !valid_zones.contains(to) {
            return Ok(vec![]);
        }

        let max_steps = self.get_steps_per_movement(state)?;
        let is_traversable = |current: &Zone, next: &Zone| -> anyhow::Result<bool> {
            Ok(self
                .get_zones_within_steps_of(state, 1, current)
                .contains(next)
                && self.can_move_between_zones(state, current, next)?)
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
                if is_traversable(&current, next)? {
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

    async fn get_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        self.base_valid_move_zones(state).await
    }

    // Returns the valid attack targets for this card.
    fn get_valid_attack_targets_from_zone(
        &self,
        state: &State,
        _ranged: bool,
        zone: &Zone,
    ) -> Vec<uuid::Uuid> {
        let attacker_region = self.get_region(state);
        let attacker_is_airborne = self.has_ability(state, &Ability::Airborne);

        state
            .cards
            .values()
            .filter(|target| {
                // Only enemy units or sites
                target.get_controller_id(state) != self.get_controller_id(state)
                    && (target.is_unit() || target.is_site())
            })
            .filter(|target| {
                target
                    .get_zone()
                    .can_be_entered_by(state, self.get_id())
                    .unwrap_or_default()
            })
            .filter(|target| {
                // Cannot attack Unattackable units, or Stealth units unless the attacker can see them.
                !target.has_ability(state, &Ability::Unattackable)
                    && (!target.has_ability(state, &Ability::Stealth)
                        || self.has_ability(state, &Ability::CanSeeStealthed))
            })
            .filter(|target| {
                let target_region = target.get_region(state);
                let target_is_airborne = target.has_ability(state, &Ability::Airborne);

                // Airborne units can attack nearby, others only adjacent
                let in_range = if attacker_is_airborne {
                    zone.is_nearby(target.get_zone())
                } else {
                    zone.is_adjacent(target.get_zone())
                };
                if !in_range {
                    return false;
                }

                match attacker_region {
                    Region::Surface => {
                        if target.is_site() {
                            // Sites are always on Surface
                            true
                        } else if target_is_airborne {
                            // Only airborne units on Surface can attack airborne units
                            attacker_is_airborne
                        } else {
                            // Both on Surface, target is not airborne
                            matches!(target_region, Region::Surface)
                        }
                    }
                    Region::Underground => matches!(target_region, Region::Underground),
                    Region::Underwater => matches!(target_region, Region::Underwater),
                    _ => false,
                }
            })
            .map(|target| *target.get_id())
            .collect()
    }

    fn can_attack(&self, state: &State) -> bool {
        self.is_unit() && !self.has_status(state, &CardStatus::Disabled) && !self.is_tapped()
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
                    .counters_from_area_modifiers(self.get_id())
                    .iter()
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

    fn has_attachments(&self, state: &State) -> anyhow::Result<bool> {
        for card in state.cards.values().filter(|c| c.get_zone().is_in_play()) {
            if card.get_bearer_id()? == Some(*self.get_id()) {
                return Ok(true);
            }
        }

        Ok(false)
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

    fn provides_no_resources(&self, _state: &State) -> anyhow::Result<bool> {
        Ok(false)
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

    fn set_controller_id(&mut self, controller_id: &PlayerId) {
        self.get_base_mut().controller_id = *controller_id;
    }

    fn set_zone(&mut self, zone: Zone) {
        self.get_base_mut().zone = zone;
    }

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    fn deathrite(&self, _state: &State, _from: &Zone) -> Vec<Effect> {
        vec![]
    }

    fn can_be_targetted_by_player(&self, state: &State, player_id: &PlayerId) -> bool {
        // A card with Stealth cannot be targeted by opponents.
        if self.has_ability(state, &Ability::Stealth) && &self.get_controller_id(state) != player_id
        {
            return false;
        }

        // Carriable artifacts carried by a Stealthed minion also cannot be targeted by opponents.
        if let Ok(Some(bearer_id)) = self.get_bearer_id() {
            let bearer = state.get_card(&bearer_id);
            if bearer.has_ability(state, &Ability::Stealth)
                && &bearer.get_controller_id(state) != player_id
            {
                return false;
            }
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

    fn is_magic(&self) -> bool {
        self.get_unit_base().is_none()
            && self.get_aura_base().is_none()
            && self.get_site_base().is_none()
            && self.get_artifact_base().is_none()
            && self.get_avatar_base().is_none()
    }

    fn is_minion(&self) -> bool {
        self.is_unit() && !self.is_avatar()
    }

    fn is_aura(&self) -> bool {
        self.get_aura_base().is_some()
    }

    fn can_cast_spell_with_id(
        &self,
        state: &State,
        spell_id: &uuid::Uuid,
        player_id: &PlayerId,
    ) -> anyhow::Result<bool> {
        if !self.get_zone().is_in_play() {
            return Ok(false);
        }

        if &self.get_controller_id(state) != player_id {
            return Ok(false);
        }

        if self.is_avatar() {
            return Ok(true);
        }

        if self.has_ability(state, &Ability::Spellcaster(None)) {
            return Ok(true);
        }

        let spell = state.get_card(spell_id);
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
        damage: Damage,
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
            ub.ability_counters.retain(|c| &c.ability != modifier);
        }
    }

    fn add_ability(&mut self, modifier: Ability) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.abilities.push(modifier);
        }
    }

    fn remove_status(&mut self, status: &CardStatus) {
        self.get_base_mut().statuses.retain(|s| s != status);
        self.get_base_mut()
            .status_counters
            .retain(|c| &c.status != status);
    }

    fn add_status(&mut self, status: CardStatus) {
        self.get_base_mut().statuses.push(status);
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

    async fn on_effect(&self, _state: &State, _effect: &Effect) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![])
    }

    async fn play_mechanic(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        let card_id = self.get_id();
        match self.get_card_type() {
            CardType::Minion => {
                let zones = self.get_valid_play_zones(state, player_id, caster_id)?;
                let prompt = "Pick a zone to play the card";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                Ok(vec![Effect::PlayCard {
                    player_id: *player_id,
                    card_id: *self.get_id(),
                    zone: zone.clone().into(),
                    spellcaster: *caster_id,
                }])
            }
            CardType::Artifact => {
                let units = self
                    .get_artifact()
                    .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                    .get_valid_attach_targets(state);
                let valid_play_zones: Vec<uuid::Uuid> = self
                    .get_valid_play_zones(state, player_id, caster_id)?
                    .into_iter()
                    .filter_map(|z| z.get_site(state))
                    .map(|s| s.get_id())
                    .cloned()
                    .collect();
                let can_be_carried = state
                    .get_card(card_id)
                    .get_artifact()
                    .ok_or(anyhow::anyhow!("artifact card does not implement artifact"))?
                    .can_be_carried();
                match can_be_carried {
                    true => {
                        const EQUIP_TO_UNIT: &str = "Equip to unit";
                        const PLAY_ATOP_SITE: &str = "Play atop site";
                        let mut options = vec![];
                        if !units.is_empty() {
                            options.push(EQUIP_TO_UNIT.to_string());
                        }
                        if !valid_play_zones.is_empty() {
                            options.push(PLAY_ATOP_SITE.to_string());
                        }

                        if options.is_empty() {
                            return Err(anyhow::anyhow!(
                                "expected at least one valid placement for artifact"
                            ));
                        }

                        let mut picked_option_idx = 0;
                        if options.len() > 1 {
                            picked_option_idx = pick_option(
                                player_id,
                                &options,
                                state,
                                "Choose how to play artifact",
                                false,
                            )
                            .await?;
                        }

                        match options[picked_option_idx].as_str() {
                            EQUIP_TO_UNIT => {
                                let picked_card_id = pick_card(
                                    *player_id,
                                    &units,
                                    state,
                                    format!("Pick a unit to attach {} to", self.get_name())
                                        .as_str(),
                                )
                                .await?;
                                let picked_card = state.get_card(&picked_card_id);
                                Ok(vec![
                                    Effect::SetBearer {
                                        card_id: *card_id,
                                        bearer_id: Some(picked_card_id),
                                    },
                                    Effect::PlayCard {
                                        player_id: *player_id,
                                        card_id: *card_id,
                                        zone: picked_card.get_zone().clone().into(),
                                        spellcaster: *caster_id,
                                    },
                                ])
                            }
                            PLAY_ATOP_SITE => {
                                let picked_site_id = pick_card(
                                    *player_id,
                                    &valid_play_zones,
                                    state,
                                    format!("Pick a site to play {} onto", self.get_name())
                                        .as_str(),
                                )
                                .await?;
                                let picked_site = state.get_card(&picked_site_id);
                                Ok(vec![Effect::PlayCard {
                                    player_id: *player_id,
                                    card_id: *card_id,
                                    zone: picked_site.get_zone().clone().into(),
                                    spellcaster: *caster_id,
                                }])
                            }
                            _ => unreachable!(),
                        }
                    }
                    false => {
                        let picked_zone = pick_zone(
                            player_id,
                            &self.get_valid_play_zones(state, player_id, caster_id)?,
                            state,
                            false,
                            "Pick a zone to play the artifact",
                        )
                        .await?;
                        Ok(vec![Effect::PlayCard {
                            player_id: *player_id,
                            card_id: *card_id,
                            zone: picked_zone.clone().into(),
                            spellcaster: *caster_id,
                        }])
                    }
                }
            }
            CardType::Aura => {
                let zones = self.get_valid_play_zones(state, player_id, caster_id)?;
                let prompt = "Pick a zone to play the aura";
                let zone = pick_zone(player_id, &zones, state, false, prompt).await?;
                Ok(vec![Effect::PlayCard {
                    player_id: *player_id,
                    card_id: *self.get_id(),
                    zone: zone.clone().into(),
                    spellcaster: *caster_id,
                }])
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

    // Returns the available actions for this card, given the current game state.
    fn get_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.has_status(state, &CardStatus::Disabled) {
            return Ok(vec![]);
        }

        // TODO: What even is this?
        if self.is_site() && self.is_flooded_site(state) {
            return Ok(vec![]);
        }

        let mut abilities = if self.is_avatar() {
            let mut abilities = self.base_avatar_activated_abilities(state)?;
            if !state.card_has_special_abilities_removed(self.get_id()) {
                abilities.extend(self.get_additional_activated_abilities(state)?);
            }
            abilities
        } else if self.is_unit() {
            let mut abilities = self.base_unit_activated_abilities(state)?;
            if !state.card_has_special_abilities_removed(self.get_id()) {
                abilities.extend(self.get_additional_activated_abilities(state)?);
            }
            abilities
        } else if state.card_has_special_abilities_removed(self.get_id()) {
            vec![]
        } else {
            self.get_additional_activated_abilities(state)?
        };

        if !state.card_has_special_abilities_removed(self.get_id()) {
            abilities.extend(state.activated_abilities_from_area_modifiers(self.get_id()));
            abilities.extend(state.activated_abilities_from_continuous_effects(self.get_id()));
        }

        Ok(abilities)
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

    // Returns the area-based ongoing effects that this card provides to other cards.
    fn area_modifiers(&self, _state: &State) -> Vec<ContinuousEffect> {
        vec![]
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

fn apply_ability_modifiers(modifiers: &mut Vec<Ability>, changes: Vec<AbilityModifier>) {
    for change in changes {
        match change {
            AbilityModifier::Grant(ability) => modifiers.push(ability),
            AbilityModifier::Remove(removal) => modifiers.retain(|m| !removal.removes(m)),
        }
    }
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
    pub abilities: Vec<Ability>,
}

pub trait ResourceProviderBaseMethods: Card {
    fn base_provided_mana(&self, state: &State) -> anyhow::Result<u8>;
    fn base_provided_affinity(&self, state: &State) -> anyhow::Result<Thresholds>;
}

impl<T: Card + ?Sized> ResourceProviderBaseMethods for T {
    fn base_provided_mana(&self, state: &State) -> anyhow::Result<u8> {
        if self.get_card_type() != CardType::Site || self.provides_no_resources(state)? {
            return Ok(0);
        }

        let mut mana = self
            .get_site_base()
            .ok_or(anyhow::anyhow!("site card has no base"))?
            .provided_mana;

        state
            .active_continuous_effects()
            .into_iter()
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

    fn base_provided_affinity(&self, state: &State) -> anyhow::Result<Thresholds> {
        if self.get_card_type() != CardType::Site || self.provides_no_resources(state)? {
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
                match state.water_site_status_from_continuous_effects(self.get_id()) {
                    Some(true) => {
                        thresholds.fire = 0;
                        thresholds.air = 0;
                        thresholds.earth = 0;
                        thresholds.water = std::cmp::max(1, thresholds.water);
                    }
                    Some(false) => thresholds.water = 0,
                    None if site.is_flooded(state)? => {
                        thresholds.fire = 0;
                        thresholds.air = 0;
                        thresholds.earth = 0;
                        thresholds.water = std::cmp::max(1, thresholds.water);
                    }
                    None => {}
                }

                state
                    .active_continuous_effects()
                    .into_iter()
                    .for_each(|ce| {
                        if let ContinuousEffect::ModifyProvidedAffinities {
                            new_affinities,
                            affected_sites,
                        } = ce
                            && affected_sites.matches(self.get_id(), state)
                        {
                            thresholds = new_affinities.clone();
                        }
                    });

                Ok(thresholds)
            }
            _ => Ok(Thresholds::ZERO),
        }
    }
}

pub trait ResourceProvider: Card {
    fn provided_mana(&self, state: &State) -> anyhow::Result<u8> {
        self.base_provided_mana(state)
    }

    fn provided_affinity(&self, state: &State) -> anyhow::Result<Thresholds> {
        self.base_provided_affinity(state)
    }
}

#[async_trait::async_trait]
pub trait Site: Card + ResourceProvider {
    fn is_valid_play_site_for(
        &self,
        state: &State,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        if card.is_site() {
            return Ok(false);
        }

        if &self.get_controller_id(state) == player_id {
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

    fn on_card_stop(&self, _state: &State, _card_id: &uuid::Uuid) -> Vec<Effect> {
        vec![]
    }

    fn base_can_be_entered_by(
        &self,
        card: &uuid::Uuid,
        _from: &Zone,
        _region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        self.get_zone().can_be_entered_by(state, card)
    }

    fn can_be_entered_by(
        &self,
        card: &uuid::Uuid,
        from: &Zone,
        region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        self.base_can_be_entered_by(card, from, region, state)
    }

    fn can_be_exited_by(
        &self,
        _card: &uuid::Uuid,
        _to: &Zone,
        _region: &Region,
        _state: &State,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn is_flooded(&self, state: &State) -> anyhow::Result<bool> {
        let temporarily_flooded = state
            .temporary_effects()
            .iter()
            .filter(|te| te.affected_cards(state).contains(self.get_id()))
            .find(|te| matches!(te, TemporaryEffect::FloodSites { .. }))
            .is_some();
        if temporarily_flooded {
            return Ok(true);
        }

        Ok(state
            .water_site_status_from_continuous_effects(self.get_id())
            .unwrap_or(false))
    }

    fn is_droughted(&self, state: &State) -> anyhow::Result<bool> {
        Ok(state.water_site_status_from_continuous_effects(self.get_id()) == Some(false))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub enum CardStatus {
    Disabled,
    Silenced,
    SummoningSickness,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Ability {
    Voidwalk,
    Airborne,
    Ranged(u8),
    Stealth,
    CanSeeStealthed,
    Lethal,
    Movement(u8),
    Leap,
    Burrowing,
    Landbound,
    Submerge,
    Spellcaster(Option<Element>),
    Charge,
    TakesNoDamageFromElement(Element),
    TakesNoDamageFromRangedStrikes,
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
    /// This unit can carry other minions. The usize parameter indicates how many minions it can
    /// carry.
    /// 0 means it can carry any number of minions, while a positive number indicates a limit on how
    /// many minions it can carry.
    CarryMinions(usize),
    SplashDamage,
    CannotDefend,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AbilityCategory {
    Keyword,
    Passive,
    Activated,
    Triggered,
}

impl Ability {
    pub fn category(&self) -> AbilityCategory {
        match self {
            Ability::Voidwalk
            | Ability::Airborne
            | Ability::Ranged(_)
            | Ability::Stealth
            | Ability::CanSeeStealthed
            | Ability::Lethal
            | Ability::Movement(_)
            | Ability::Leap
            | Ability::Burrowing
            | Ability::Landbound
            | Ability::Submerge
            | Ability::Spellcaster(_)
            | Ability::Charge
            | Ability::Immobile
            | Ability::TakesNoDamageFromElement(_)
            | Ability::TakesNoDamageFromRangedStrikes
            | Ability::Waterbound
            | Ability::Lifesteal
            | Ability::FirstStrike
            | Ability::Unattackable
            | Ability::Uninterceptable
            | Ability::Oversized
            | Ability::LethalTarget
            | Ability::CarryMinions(_)
            | Ability::SplashDamage
            | Ability::CannotDefend => AbilityCategory::Keyword,
        }
    }

    pub fn is_keyword_ability(&self) -> bool {
        self.category() == AbilityCategory::Keyword
    }

    pub fn is_card_ability(&self) -> bool {
        true
    }

    pub fn is_special_ability(&self) -> bool {
        self.is_card_ability()
    }

    pub fn persists_while_disabled(&self) -> bool {
        matches!(self, Ability::Landbound | Ability::Waterbound)
    }
}

#[derive(Debug, Default, Clone)]
pub struct UnitBase {
    pub power: u16,
    pub toughness: u16,
    pub abilities: Vec<Ability>,
    pub damage: u16,
    pub power_counters: Vec<Counter>,
    pub ability_counters: Vec<AbilityCounter>,
    pub types: Vec<MinionType>,
    pub carried_by: Option<uuid::Uuid>,
    pub tapped: bool,
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

#[derive(Debug, Default, Clone)]
pub struct ArtifactBase {
    pub types: Vec<ArtifactType>,
    pub tapped: bool,
}

pub trait Artifact: Card {
    fn can_be_carried(&self) -> bool {
        let artifact_types = &self
            .get_artifact_base()
            .expect("artifact to have an artifact base")
            .types;

        // Automatons and Monuments cannot be carried
        !artifact_types.contains(&ArtifactType::Automaton)
            && !artifact_types.contains(&ArtifactType::Monument)
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
    pub statuses: Vec<CardStatus>,
    pub status_counters: Vec<StatusCounter>,
    // In the case of artifacts, bearer is the id of the card that has the artifact equipped. This
    // field can also be used for units to track when another unit is carrying them (e.g. a unit
    // being carried by Beast of Burden).
    pub bearer: Option<uuid::Uuid>,
    pub rarity: Rarity,
    pub edition: Edition,
    pub is_token: bool,
    pub needs_explicit_spellcaster: bool,
}

impl Default for CardBase {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            owner_id: PlayerId::default(),
            controller_id: PlayerId::default(),
            zone: Zone::default(),
            costs: Costs::default(),
            statuses: vec![],
            status_counters: vec![],
            bearer: None,
            rarity: Rarity::default(),
            edition: Edition::default(),
            is_token: false,
            needs_explicit_spellcaster: false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AuraBase {
    pub tapped: bool,
}

pub trait Aura: Card {
    fn should_dispell(&self, _state: &State) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn get_affected_zones(&self, state: &State) -> Vec<Zone> {
        self.base_get_affected_zones(state)
    }
}

#[derive(Debug, Clone)]
pub struct Damage {
    pub amount: u16,
    pub is_attack: bool,
    pub is_ranged: bool,
    pub is_lethal: bool,
    pub is_strike: bool,
}

impl std::ops::Mul<u16> for &Damage {
    type Output = Damage;

    fn mul(self, rhs: u16) -> Self::Output {
        Damage {
            amount: self.amount.saturating_mul(rhs),
            is_attack: self.is_attack,
            is_ranged: self.is_ranged,
            is_lethal: self.is_lethal,
            is_strike: self.is_strike,
        }
    }
}

impl std::ops::Mul<u16> for Damage {
    type Output = Self;

    fn mul(self, rhs: u16) -> Self::Output {
        Self {
            amount: self.amount.saturating_mul(rhs),
            is_attack: self.is_attack,
            is_ranged: self.is_ranged,
            is_lethal: self.is_lethal,
            is_strike: self.is_strike,
        }
    }
}

impl Damage {
    pub fn basic(amount: u16) -> Self {
        Self {
            amount,
            is_attack: false,
            is_ranged: false,
            is_lethal: false,
            is_strike: false,
        }
    }

    pub fn attack(amount: u16) -> Self {
        Self {
            amount,
            is_attack: true,
            is_ranged: false,
            is_lethal: false,
            is_strike: false,
        }
    }

    pub fn strike(amount: u16, is_ranged: bool) -> Self {
        Self {
            amount,
            is_attack: true,
            is_ranged,
            is_lethal: false,
            is_strike: true,
        }
    }

    pub fn lethal(amount: u16) -> Self {
        Self {
            amount,
            is_attack: false,
            is_ranged: false,
            is_lethal: true,
            is_strike: false,
        }
    }
}

/// CardBaseMethods are the default implementations of certain card behaviours, like calculating
/// power and toughness, or determining which zones are affected by an aura. These methods can be
/// called by specific card types in their own implementations of the corresponding methods, to get
/// the default behaviour and then modify it as needed. For example, a unit card can call
/// `base_get_power` in its implementation of `get_power` to get the default power calculation, and
/// then apply additional modifiers based on its own abilities or effects.
///
/// These methods should not be overridden by specific card types, as they provide the base
/// behaviour for all cards. Instead, specific card types should override the public methods defined
/// in the `Card` trait, and can call these base methods to get the default behaviour when needed.
#[async_trait::async_trait]
pub trait CardBaseMethods: Card {
    fn base_get_affected_zones(&self, state: &State) -> Vec<Zone>;
    fn base_get_power(&self, state: &State) -> Option<u16>;
    fn base_get_abilities(&self, state: &State) -> Vec<Ability>;
    fn base_take_damage(
        &mut self,
        state: &State,
        from: &uuid::Uuid,
        damage: Damage,
    ) -> anyhow::Result<Vec<Effect>>;
    fn base_site_on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>>;
    async fn base_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>>;
    fn base_avatar_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>>;
    fn base_unit_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>>;
    fn base_get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>>;
}

#[async_trait::async_trait]
impl<T: Card + ?Sized> CardBaseMethods for T {
    fn base_get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
        _caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Zone>> {
        Ok(Zone::all_board()
            .into_iter()
            .filter(|z| {
                let costs = state
                    .get_effective_costs(self.get_id(), Some(z), player_id)
                    .unwrap_or_default();
                let can_afford = costs.can_afford(state, player_id).unwrap_or_default();
                if !can_afford {
                    return false;
                }

                z.is_valid_play_zone_for(state, self.get_id(), player_id)
                    .unwrap_or_default()
            })
            .collect::<Vec<Zone>>())
    }

    fn base_get_affected_zones(&self, _state: &State) -> Vec<Zone> {
        match self.get_zone() {
            z @ Zone::Location(_, _) => vec![z.clone()],
            Zone::Intersection(locs, region) => {
                let mut zones = Vec::new();
                for sq in locs {
                    zones.push(Zone::Location(*sq, region.clone()));
                }
                zones
            }
            _ => vec![],
        }
    }

    fn base_get_power(&self, state: &State) -> Option<u16> {
        match self.get_unit_base() {
            Some(base) => {
                let mut power = base.power;
                for counter in &base.power_counters {
                    power = power.saturating_add_signed(counter.power);
                }

                power = power
                    .saturating_add_signed(state.power_diff_from_continuous_effects(self.get_id()));

                let power_counters: i16 = state
                    .counters_from_area_modifiers(self.get_id())
                    .iter()
                    .map(|counter| counter.power)
                    .sum();
                power = power.saturating_add_signed(power_counters);

                Some(power)
            }
            None => None,
        }
    }

    fn base_get_abilities(&self, state: &State) -> Vec<Ability> {
        match self.get_card_type() {
            CardType::Minion | CardType::Avatar => {
                let base = self
                    .get_unit_base()
                    .expect("minions and avatars to have a unit base");
                let mut modifiers = base.abilities.clone();
                for counter in &base.ability_counters {
                    modifiers.push(counter.ability.clone());
                }

                apply_ability_modifiers(
                    &mut modifiers,
                    state.ability_modifiers_from_area_modifiers(self.get_id()),
                );

                // Units that can carry other units confer Airborne, Burrowing, Submerge, and/or
                // Voidwalk to the carried units while they're carried.
                if let Some(bearer_id) = self.get_bearer_id().ok().flatten() {
                    let bearer = state.get_card(&bearer_id);
                    for ability in [
                        Ability::Airborne,
                        Ability::Burrowing,
                        Ability::Submerge,
                        Ability::Voidwalk,
                    ] {
                        if bearer
                            .get_abilities(state)
                            .is_ok_and(|abilities| abilities.contains(&ability))
                        {
                            modifiers.push(ability);
                        }
                    }
                }

                modifiers.extend(state.granted_abilities_from_continuous_effects(self.get_id()));

                if self.has_status(state, &CardStatus::Silenced) {
                    modifiers.retain(|ability| !ability.is_special_ability());
                }
                if self.has_status(state, &CardStatus::Disabled) {
                    modifiers.retain(|ability| ability.persists_while_disabled());
                }

                modifiers
            }
            CardType::Site => {
                let base = self.get_site_base().expect("site to have a site base");
                let mut modifiers = base.abilities.clone();
                apply_ability_modifiers(
                    &mut modifiers,
                    state.ability_modifiers_from_area_modifiers(self.get_id()),
                );
                modifiers.extend(state.granted_abilities_from_continuous_effects(self.get_id()));

                if self.has_status(state, &CardStatus::Silenced)
                    || self.has_status(state, &CardStatus::Disabled)
                {
                    modifiers.retain(|ability| !ability.is_special_ability());
                }

                modifiers
            }
            _ => vec![],
        }
    }

    fn base_take_damage(
        &mut self,
        state: &State,
        from: &uuid::Uuid,
        damage: Damage,
    ) -> anyhow::Result<Vec<Effect>> {
        if self.has_ability(state, &Ability::TakesNoDamageFromRangedStrikes) && damage.is_ranged {
            return Ok(vec![]);
        }

        let dealer = state.get_card(from);
        if dealer.get_card_type() == CardType::Magic
            && state
                .active_continuous_effects()
                .into_iter()
                .any(|ce| match ce {
                    ContinuousEffect::PreventDamageFromMagic { affected_cards } => {
                        affected_cards.matches(self.get_id(), state)
                    }
                    _ => false,
                })
        {
            return Ok(vec![]);
        }

        let elements = dealer.get_elements(state)?;
        for element in elements {
            if self.has_ability(state, &Ability::TakesNoDamageFromElement(element)) {
                return Ok(vec![]);
            }
        }

        let reduced_damage = state
            .active_continuous_effects()
            .into_iter()
            .filter_map(|ce| match ce {
                ContinuousEffect::ReduceDamageTaken {
                    amount,
                    affected_cards,
                } if affected_cards.matches(self.get_id(), state) => Some(amount),
                _ => None,
            })
            .fold(damage.amount, |remaining, amount| {
                remaining.saturating_sub(*amount)
            });

        match self.get_card_type() {
            CardType::Minion => {
                // Check LethalTarget before the mutable borrow of unit_base.
                let has_lethal_target = self.get_unit_base().is_some_and(|ub| {
                    ub.abilities.contains(&Ability::LethalTarget)
                        || ub
                            .ability_counters
                            .iter()
                            .any(|c| c.ability == Ability::LethalTarget)
                }) || state
                    .granted_abilities_from_continuous_effects(self.get_id())
                    .contains(&Ability::LethalTarget);

                let ub = self
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("unit card has no unit base"))?;
                ub.damage += reduced_damage;

                let mut effects = vec![];
                let dealer = state.get_card(from);
                let killer_id = if dealer.is_magic() {
                    state.find_caster(from).expect("magic to have a caster")
                } else {
                    *from
                };
                // Zero damage is not any damage at all (rulebook). Only apply kill conditions
                // when actual damage was dealt.
                if reduced_damage > 0
                    && (ub.damage >= self.get_toughness(state).unwrap_or(0)
                        || damage.is_lethal
                        || dealer.has_ability(state, &Ability::Lethal)
                        || has_lethal_target)
                {
                    effects.push(Effect::KillMinion {
                        card_id: *self.get_id(),
                        killer_id,
                        from_attack: damage.is_attack,
                    });
                }

                // Lifesteal: if the defender is a unit, heal the attacker's controller.
                let defender = state.get_card(self.get_id());
                if dealer.has_ability(state, &Ability::Lifesteal) && defender.is_unit() {
                    let controller_id = dealer.get_controller_id(state);
                    if let Ok(avatar_id) = state.get_player_avatar_id(&controller_id) {
                        let heal = dealer.get_power(state)?.unwrap_or(0);
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

                if ab.deaths_door && ab.can_die && reduced_damage > 0 {
                    return Ok(vec![Effect::PlayerLost {
                        player_id: self.get_controller_id(state),
                    }]);
                }

                let ub = self
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("unit card has no unit base"))?;
                ub.damage += reduced_damage;

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
                let controller_id = self.get_controller_id(state);
                let avatar_id = state.get_player_avatar_id(&controller_id)?;
                let avatar = state.get_card(&avatar_id);
                let unit_base = avatar
                    .get_unit_base()
                    .ok_or(anyhow::anyhow!("avatar has no unit base"))?;
                let current_life = unit_base.toughness.saturating_sub(unit_base.damage);

                // Attacking sites causes life loss, not damage.
                Ok(vec![Effect::SetAvatarLife {
                    player_id: controller_id,
                    life: current_life.saturating_sub(reduced_damage),
                }])
            }
            _ => Ok(vec![]),
        }
    }

    fn base_site_on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let site_base = self
            .get_site()
            .ok_or(anyhow::anyhow!("site card has no site base"))?;
        Ok(vec![Effect::AdjustMana {
            player_id: *self.get_owner_id(),
            mana: site_base.provided_mana(state)? as i8,
        }])
    }

    async fn base_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        // If the card is not a unit, it might be an aura, in which case the result of
        // get_zones_within_steps should be returned as is.
        if !self.is_unit() {
            return Ok(self.get_zones_within_steps(state, self.get_steps_per_movement(state)?));
        }

        let mut zones: Vec<Zone> = vec![];
        for zone in &self.get_zones_within_steps(state, self.get_steps_per_movement(state)?) {
            if !zone.can_be_entered_by(state, self.get_id())? {
                continue;
            }

            if zone.get_site(state).is_none() {
                if self.has_ability(state, &Ability::Voidwalk)
                    || is_continuously_connected_zone(state, self.get_id(), zone)
                {
                    zones.push(zone.clone());
                }
                continue;
            }

            // Oversized units may only move to intersection zones where all 4 sub-zones have sites.
            if self.is_oversized(state)
                && let Zone::Intersection(sqs, region) = zone
                && sqs.iter().all(|sq| {
                    Zone::Location(*sq, region.clone())
                        .get_site(state)
                        .is_some()
                })
            {
                zones.push(zone.clone());
                continue;
            };

            if zone.get_site(state).is_none() {
                continue;
            };

            zones.push(zone.clone());
        }

        for ce in state.active_continuous_effects() {
            if let ContinuousEffect::RestrictMoveToZones {
                affected_cards,
                allowed_zones,
            } = ce
                && affected_cards.matches(self.get_id(), state)
            {
                zones.retain(|zone| allowed_zones.contains(zone));
            }
        }

        Ok(zones)
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
            .values()
            .filter(|c| c.get_controller_id(state) == self.get_controller_id(state))
            .filter(|c| c.is_site())
            .filter(|c| matches!(c.get_zone(), Zone::Hand))
            .count()
            > 0
        {
            activated_abilities.push(Box::new(AvatarAction::PlaySite));
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

        let unborne_artifacts: Vec<(uuid::Uuid, String)> = CardQuery::new()
            .artifacts()
            .in_zone(self.get_zone())
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

        let carry_minions_ability = self
            .get_abilities(state)?
            .into_iter()
            .find(|a| matches!(a, Ability::CarryMinions(_)));
        if let Some(Ability::CarryMinions(n)) = carry_minions_ability {
            let carried_minions = CardQuery::new()
                .minions()
                .carried_by(self.get_id())
                .all(state);
            let carriable_minions = CardQuery::new()
                .minions()
                .controlled_by(&self.get_controller_id(state))
                .in_zone(self.get_zone())
                .not_carried()
                .all(state);
            let can_carry_more = carried_minions.len() < n || n == 0;
            if can_carry_more && !carriable_minions.is_empty() {
                activated_abilities.push(Box::new(UnitAction::PickUpMinion));
            }
            if !carried_minions.is_empty() {
                activated_abilities.push(Box::new(UnitAction::DropMinion));
            }
        }

        Ok(activated_abilities)
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

fn zones_cross_border(from: &Zone, to: &Zone, border: &Zone) -> bool {
    let (Some(from_square), Some(to_square)) = (from.get_square(), to.get_square()) else {
        return false;
    };

    match border {
        Zone::Intersection(squares, _) => {
            squares.contains(&from_square) && squares.contains(&to_square)
        }
        _ => false,
    }
}

fn temporarily_connected_sites(state: &State, card_id: &uuid::Uuid, zone: &Zone) -> Vec<Zone> {
    state
        .temporary_effects()
        .iter()
        .filter_map(|effect| match effect {
            TemporaryEffect::ConnectSites {
                sites,
                affected_cards,
                ..
            } if affected_cards.matches(card_id, state) && sites.contains(zone) => {
                Some(sites.iter().filter(move |site| *site != zone).cloned())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

fn continuously_connected_zones(state: &State, card_id: &uuid::Uuid) -> Vec<Zone> {
    state
        .active_continuous_effects()
        .into_iter()
        .filter_map(|effect| match effect {
            ContinuousEffect::ConnectZones {
                connected_zones,
                affected_cards,
            } if affected_cards.matches(card_id, state) => Some(connected_zones.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn is_continuously_connected_zone(state: &State, card_id: &uuid::Uuid, zone: &Zone) -> bool {
    state
        .active_continuous_effects()
        .into_iter()
        .any(|effect| match effect {
            ContinuousEffect::ConnectZones {
                connected_zones,
                affected_cards,
            } => affected_cards.matches(card_id, state) && connected_zones.contains(zone),
            _ => false,
        })
}

fn leap_destinations(state: &State, zone: &Zone) -> Vec<Zone> {
    let Some(square) = zone.get_square() else {
        return vec![];
    };
    let Zone::Location(_, region) = zone else {
        return vec![];
    };
    let col = ((square - 1) % 5) as i8;
    let row = ((square - 1) / 5) as i8;

    [(0, -1), (0, 1), (-1, 0), (1, 0)]
        .into_iter()
        .filter_map(|(dc, dr)| {
            let middle_col = col + dc;
            let middle_row = row + dr;
            let landing_col = col + (dc * 2);
            let landing_row = row + (dr * 2);
            if !(0..5).contains(&middle_col)
                || !(0..4).contains(&middle_row)
                || !(0..5).contains(&landing_col)
                || !(0..4).contains(&landing_row)
            {
                return None;
            }
            let middle_square = (middle_row * 5 + middle_col + 1) as u8;
            let middle = Zone::Location(middle_square, region.clone());
            middle.get_site(state)?;
            let landing_square = (landing_row * 5 + landing_col + 1) as u8;
            Some(Zone::Location(landing_square, region.clone()))
        })
        .collect()
}
