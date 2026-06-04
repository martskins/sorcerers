use crate::{
    card::{Ability, Card, CardData, CardStatus, CardType, Costs, DodgeRoll, SiteType, UnitBase},
    deck::Deck,
    effect::Counter,
    effect::{Effect, EffectCallback, EffectEngine, EffectState},
    game::{
        ActivatedAbility, CardId, PlayerId, Resources, Thresholds, ThresholdsDiff, pick_zone,
        yes_or_no,
    },
    networking::message::{ClientMessage, OngoingEffectData, ServerMessage},
    query::{CardQuery, EffectQuery, LocationQuery, ZoneQuery},
    zone::Zone,
};
use async_channel::{Receiver, Sender};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::RwLock,
};

pub use crate::effect::{DeferredEffect, LoggedEffect, TemporaryEffect};

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
pub struct AreaModifierIndex {
    pub ability_modifiers: HashMap<CardId, Vec<AbilityModifier>>,
    pub grants_statuses: HashMap<CardId, Vec<CardStatus>>,
    pub grants_activated_abilities: HashMap<CardId, Vec<Box<dyn ActivatedAbility>>>,
    pub grants_counters: HashMap<CardId, Vec<Counter>>,
}

#[derive(Debug, Clone)]
pub enum AbilityModifier {
    Grant(Ability),
    Remove(AbilityRemoval),
}

#[derive(Debug, Clone)]
pub enum AbilityRemoval {
    Exact(Vec<Ability>),
    AllAbilities,
    AllAbilitiesExcept(Vec<Ability>),
    SpecialAbilities,
}

impl AbilityRemoval {
    pub fn exact(ability: Ability) -> Self {
        Self::Exact(vec![ability])
    }

    pub fn is_silence(&self) -> bool {
        matches!(self, Self::SpecialAbilities)
    }

    pub fn removes_special_abilities(&self) -> bool {
        matches!(
            self,
            Self::AllAbilities | Self::AllAbilitiesExcept(_) | Self::SpecialAbilities
        )
    }

    pub fn removes(&self, ability: &Ability) -> bool {
        match self {
            Self::Exact(abilities) => abilities.contains(ability),
            Self::AllAbilities => ability.is_card_ability(),
            Self::AllAbilitiesExcept(exceptions) => {
                ability.is_card_ability() && !exceptions.contains(ability)
            }
            Self::SpecialAbilities => ability.is_special_ability(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AffinityModifier {
    Set(Thresholds),
    AddMinimum(Thresholds),
    Remove(Thresholds),
}

impl AffinityModifier {
    pub fn apply(&self, affinities: &mut Thresholds) {
        match self {
            Self::Set(new_affinities) => *affinities = new_affinities.clone(),
            Self::AddMinimum(minimums) => {
                affinities.fire = affinities.fire.max(minimums.fire);
                affinities.air = affinities.air.max(minimums.air);
                affinities.earth = affinities.earth.max(minimums.earth);
                affinities.water = affinities.water.max(minimums.water);
            }
            Self::Remove(to_remove) => {
                affinities.fire = affinities.fire.saturating_sub(to_remove.fire);
                affinities.air = affinities.air.saturating_sub(to_remove.air);
                affinities.earth = affinities.earth.saturating_sub(to_remove.earth);
                affinities.water = affinities.water.saturating_sub(to_remove.water);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimedOngoingEffect {
    pub effect: OngoingEffect,
    pub source: Option<CardId>,
    pub timestamp: u64,
}

#[derive(Debug, Default, Clone)]
pub struct OngoingEffectIndex {
    pub grants_statuses: HashMap<CardId, Vec<CardStatus>>,
    pub grants_activated_abilities: HashMap<CardId, Vec<Box<dyn ActivatedAbility>>>,
    pub power_diffs: HashMap<CardId, i16>,
}

impl OngoingEffectIndex {
    fn build(state: &State) -> Self {
        let mut index = Self::default();
        let mut inactive_sources = HashSet::new();

        for effect in state.ordered_ongoing_effects() {
            if effect
                .source
                .is_some_and(|source_id| inactive_sources.contains(&source_id))
            {
                continue;
            }

            match &effect.effect {
                OngoingEffect::GrantStatus {
                    status,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .grants_statuses
                            .entry(card_id)
                            .or_default()
                            .push(status.clone());
                        if matches!(status, CardStatus::Disabled | CardStatus::Silenced) {
                            inactive_sources.insert(card_id);
                        }
                    }
                }
                OngoingEffect::RemoveAbilities {
                    removal,
                    affected_cards,
                } => {
                    if removal.removes_special_abilities() {
                        for card_id in affected_cards.all(state) {
                            inactive_sources.insert(card_id);
                        }
                    }
                }
                OngoingEffect::ModifyPower {
                    power_diff,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        *index.power_diffs.entry(card_id).or_default() += *power_diff;
                    }
                }
                OngoingEffect::ModifyPowerForEach {
                    power_per_card,
                    affected_cards,
                    matching_cards,
                } => {
                    let power_diff = *power_per_card * matching_cards.all(state).len() as i16;
                    if power_diff != 0 {
                        for card_id in affected_cards.all(state) {
                            *index.power_diffs.entry(card_id).or_default() += power_diff;
                        }
                    }
                }
                _ => {}
            }
        }

        index
    }
}

impl AreaModifierIndex {
    fn build(state: &State) -> Self {
        let mut index = Self::default();
        let mut inactive_sources = HashSet::new();
        let mut flooded_removals = Vec::new();

        for effect in state.ordered_ongoing_effects() {
            if effect
                .source
                .is_some_and(|source_id| inactive_sources.contains(&source_id))
            {
                continue;
            }

            match &effect.effect {
                OngoingEffect::GrantAbility {
                    ability,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .ability_modifiers
                            .entry(card_id)
                            .or_default()
                            .push(AbilityModifier::Grant(ability.clone()));
                    }
                }
                OngoingEffect::GrantStatus {
                    status,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .grants_statuses
                            .entry(card_id)
                            .or_default()
                            .push(status.clone());
                        if matches!(status, CardStatus::Disabled | CardStatus::Silenced) {
                            inactive_sources.insert(card_id);
                        }
                    }
                }
                OngoingEffect::RemoveAbilities {
                    removal,
                    affected_cards,
                } => {
                    let affected_cards = affected_cards.all(state);
                    if removal.removes_special_abilities() {
                        inactive_sources.extend(affected_cards.iter().copied());
                    }
                    if removal.removes(&Ability::Flooded) {
                        flooded_removals.extend(affected_cards.iter().copied());
                    }
                    for card_id in affected_cards {
                        index
                            .ability_modifiers
                            .entry(card_id)
                            .or_default()
                            .push(AbilityModifier::Remove(removal.clone()));
                    }
                }
                OngoingEffect::GrantActivatedAbility {
                    ability,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .grants_activated_abilities
                            .entry(card_id)
                            .or_default()
                            .push(ability.clone());
                    }
                }
                OngoingEffect::GrantCounter {
                    counter,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .grants_counters
                            .entry(card_id)
                            .or_default()
                            .push(counter.clone());
                    }
                }
                _ => {}
            }
        }

        for effect in state.temporary_effects() {
            if let TemporaryEffect::GrantAbility {
                ability,
                affected_cards,
                ..
            } = effect
            {
                for card_id in affected_cards.all(state) {
                    index
                        .ability_modifiers
                        .entry(card_id)
                        .or_default()
                        .push(AbilityModifier::Grant(ability.clone()));
                }
            }
        }

        for card_id in flooded_removals {
            index
                .ability_modifiers
                .entry(card_id)
                .or_default()
                .push(AbilityModifier::Remove(AbilityRemoval::exact(
                    Ability::Flooded,
                )));
        }

        index
    }
}

#[derive(Debug, Default)]
pub struct StateRuntimeCache {
    area_modifier_index: RwLock<Option<AreaModifierIndex>>,
    continuous_effect_index: RwLock<Option<OngoingEffectIndex>>,
}

impl Clone for StateRuntimeCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl StateRuntimeCache {
    fn clear(&self) {
        self.area_modifier_index
            .write()
            .expect("area modifier index lock should not be poisoned")
            .take();
        self.continuous_effect_index
            .write()
            .expect("continuous effect index lock should not be poisoned")
            .take();
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum OngoingEffect {
    ControllerOverride {
        controller_id: PlayerId,
        affected_cards: CardQuery,
    },
    ModifyPower {
        power_diff: i16,
        affected_cards: CardQuery,
    },
    ModifyPowerForEach {
        power_per_card: i16,
        affected_cards: CardQuery,
        matching_cards: CardQuery,
    },
    MakeZoneUnvisitable {
        affected_zone: Zone,
        affected_cards: CardQuery,
    },
    DoubleDamageTaken {
        affected_cards: CardQuery,
        except_strikes: bool,
    },
    ReduceDamageTaken {
        amount: u16,
        affected_cards: CardQuery,
    },
    PreventDamageFromMagic {
        affected_cards: CardQuery,
    },
    RestrictMoveToZones {
        affected_cards: CardQuery,
        allowed_zones: Vec<Zone>,
    },
    BlockMovementThrough {
        border: Zone,
        affected_cards: CardQuery,
    },
    ConnectTopBottomEdges {
        affected_cards: CardQuery,
    },
    ConnectLeftRightEdges {
        affected_cards: CardQuery,
    },
    ConnectZones {
        connected_zones: Vec<Zone>,
        affected_cards: CardQuery,
    },
    ChangeSiteType {
        site_type: SiteType,
        affected_sites: CardQuery,
    },
    GrantAbility {
        ability: Ability,
        affected_cards: CardQuery,
    },
    GrantStatus {
        status: CardStatus,
        affected_cards: CardQuery,
    },
    RemoveAbilities {
        removal: AbilityRemoval,
        affected_cards: CardQuery,
    },
    GrantActivatedAbility {
        ability: Box<dyn ActivatedAbility>,
        affected_cards: CardQuery,
    },
    GrantCounter {
        counter: Counter,
        affected_cards: CardQuery,
    },
    ModifyProvidedAffinities {
        modifier: AffinityModifier,
        affected_sites: CardQuery,
    },
    ModifyProvidedMana {
        mana_diff: i8,
        affected_cards: CardQuery,
    },
    OverrideValidPlayZone {
        affected_zones: ZoneQuery,
        affected_cards: CardQuery,
    },
    ModifyManaCost {
        mana_diff: i8,
        affected_cards: CardQuery,
        zones: Option<ZoneQuery>,
    },
    TriggeredEffect {
        trigger_on_effect: EffectQuery,
        on_effect: EffectCallback,
    },
}

impl OngoingEffect {
    fn display_description(&self) -> String {
        match self {
            Self::ControllerOverride { controller_id, .. } => {
                format!("Controller becomes {}", controller_id)
            }
            Self::ModifyPower { power_diff, .. } => {
                format!("Power {:+}", power_diff)
            }
            Self::ModifyPowerForEach { power_per_card, .. } => {
                format!("Power {:+} for each matching card", power_per_card)
            }
            Self::MakeZoneUnvisitable { affected_zone, .. } => {
                format!("Makes {} unvisitable", affected_zone)
            }
            Self::DoubleDamageTaken { except_strikes, .. } if *except_strikes => {
                "Doubles non-strike damage taken".to_string()
            }
            Self::DoubleDamageTaken { .. } => "Doubles damage taken".to_string(),
            Self::ReduceDamageTaken { amount, .. } => {
                format!("Reduces damage taken by {}", amount)
            }
            Self::PreventDamageFromMagic { .. } => "Prevents magic damage".to_string(),
            Self::RestrictMoveToZones { allowed_zones, .. } => {
                format!("Restricts movement to {} zones", allowed_zones.len())
            }
            Self::BlockMovementThrough { border, .. } => {
                format!("Blocks movement through {}", border)
            }
            Self::ConnectTopBottomEdges { .. } => "Connects top and bottom edges".to_string(),
            Self::ConnectLeftRightEdges { .. } => "Connects left and right edges".to_string(),
            Self::ConnectZones {
                connected_zones, ..
            } => format!("Connects {} zones", connected_zones.len()),
            Self::ChangeSiteType { site_type, .. } => {
                format!("Changes site type to {:?}", site_type)
            }
            Self::GrantAbility { ability, .. } => {
                format!("Grants {:?}", ability)
            }
            Self::GrantStatus { status, .. } => {
                format!("Grants {:?} status", status)
            }
            Self::RemoveAbilities { removal, .. } => match removal {
                AbilityRemoval::Exact(abilities) => {
                    format!("Removes {} abilities", abilities.len())
                }
                AbilityRemoval::AllAbilities => "Removes all abilities".to_string(),
                AbilityRemoval::AllAbilitiesExcept(exceptions) => {
                    format!("Removes all abilities except {}", exceptions.len())
                }
                AbilityRemoval::SpecialAbilities => "Removes special abilities".to_string(),
            },
            Self::GrantActivatedAbility { ability, .. } => {
                format!("Grants {}", ability.get_name())
            }
            Self::GrantCounter { counter, .. } => {
                format!("Grants {:?} counter", counter)
            }
            Self::ModifyProvidedAffinities { modifier, .. } => {
                format!("Modifies provided affinities: {:?}", modifier)
            }
            Self::ModifyProvidedMana { mana_diff, .. } => {
                format!("Provided mana {:+}", mana_diff)
            }
            Self::OverrideValidPlayZone { .. } => "Overrides valid play zones".to_string(),
            Self::ModifyManaCost { mana_diff, .. } => {
                format!("Mana cost {:+}", mana_diff)
            }
            Self::TriggeredEffect { .. } => "Triggered ongoing effect".to_string(),
        }
    }

    fn affected_card_ids(&self, state: &State) -> Vec<CardId> {
        match self {
            Self::ControllerOverride { affected_cards, .. }
            | Self::ModifyPower { affected_cards, .. }
            | Self::ModifyPowerForEach { affected_cards, .. }
            | Self::MakeZoneUnvisitable { affected_cards, .. }
            | Self::DoubleDamageTaken { affected_cards, .. }
            | Self::ReduceDamageTaken { affected_cards, .. }
            | Self::PreventDamageFromMagic { affected_cards, .. }
            | Self::RestrictMoveToZones { affected_cards, .. }
            | Self::BlockMovementThrough { affected_cards, .. }
            | Self::ConnectTopBottomEdges { affected_cards, .. }
            | Self::ConnectLeftRightEdges { affected_cards, .. }
            | Self::ConnectZones { affected_cards, .. }
            | Self::GrantAbility { affected_cards, .. }
            | Self::GrantStatus { affected_cards, .. }
            | Self::RemoveAbilities { affected_cards, .. }
            | Self::GrantActivatedAbility { affected_cards, .. }
            | Self::GrantCounter { affected_cards, .. }
            | Self::ModifyProvidedMana { affected_cards, .. }
            | Self::OverrideValidPlayZone { affected_cards, .. }
            | Self::ModifyManaCost { affected_cards, .. } => affected_cards.all(state),
            Self::ChangeSiteType { affected_sites, .. }
            | Self::ModifyProvidedAffinities { affected_sites, .. } => affected_sites.all(state),
            Self::TriggeredEffect { .. } => Vec::new(),
        }
    }

    fn explicit_affected_zones(&self, state: &State) -> Vec<Zone> {
        match self {
            Self::MakeZoneUnvisitable { affected_zone, .. } => vec![affected_zone.clone()],
            Self::RestrictMoveToZones { allowed_zones, .. } => allowed_zones.clone(),
            Self::BlockMovementThrough { border, .. } => vec![border.clone()],
            Self::ConnectZones {
                connected_zones, ..
            } => connected_zones.clone(),
            Self::OverrideValidPlayZone { affected_zones, .. } => affected_zones.options(state),
            Self::ModifyManaCost {
                zones: Some(zones), ..
            } => zones.options(state),
            _ => Vec::new(),
        }
    }
}

impl std::fmt::Debug for OngoingEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ControllerOverride { .. } => f.debug_struct("ControllerOverride").finish(),
            Self::ModifyPower { power_diff, .. } => f
                .debug_struct("ModifyPower")
                .field("power_diff", power_diff)
                .finish(),
            Self::ModifyPowerForEach { power_per_card, .. } => f
                .debug_struct("ModifyPowerForEach")
                .field("power_per_card", power_per_card)
                .finish(),
            Self::MakeZoneUnvisitable { affected_zone, .. } => f
                .debug_struct("MakeZoneUnvisitable")
                .field("affected_zone", affected_zone)
                .finish(),
            Self::DoubleDamageTaken { except_strikes, .. } => f
                .debug_struct("DoubleDamageTaken")
                .field("except_strikes", except_strikes)
                .finish(),
            Self::ReduceDamageTaken { amount, .. } => f
                .debug_struct("ReduceDamageTaken")
                .field("amount", amount)
                .finish(),
            Self::PreventDamageFromMagic { .. } => {
                f.debug_struct("PreventDamageFromMagic").finish()
            }
            Self::RestrictMoveToZones { allowed_zones, .. } => f
                .debug_struct("RestrictMoveToZones")
                .field("allowed_zones", allowed_zones)
                .finish(),
            Self::BlockMovementThrough { border, .. } => f
                .debug_struct("BlockMovementThrough")
                .field("border", border)
                .finish(),
            Self::ConnectTopBottomEdges { .. } => f.debug_struct("ConnectTopBottomEdges").finish(),
            Self::ConnectLeftRightEdges { .. } => f.debug_struct("ConnectLeftRightEdges").finish(),
            Self::ConnectZones {
                connected_zones, ..
            } => f
                .debug_struct("ConnectZones")
                .field("connected_zones", connected_zones)
                .finish(),
            Self::ChangeSiteType { site_type, .. } => f
                .debug_struct("ChangeSiteType")
                .field("site_type", site_type)
                .finish(),
            Self::GrantAbility { ability, .. } => f
                .debug_struct("GrantAbility")
                .field("ability", ability)
                .finish(),
            Self::GrantStatus { status, .. } => f
                .debug_struct("GrantStatus")
                .field("status", status)
                .finish(),
            Self::RemoveAbilities { removal, .. } => f
                .debug_struct("RemoveAbilities")
                .field("removal", removal)
                .finish(),
            Self::GrantActivatedAbility { .. } => f.debug_struct("GrantActivatedAbility").finish(),
            Self::GrantCounter { counter, .. } => f
                .debug_struct("GrantCounter")
                .field("counter", counter)
                .finish(),
            Self::ModifyProvidedAffinities { modifier, .. } => f
                .debug_struct("ModifyProvidedAffinities")
                .field("modifier", modifier)
                .finish(),
            Self::ModifyProvidedMana { mana_diff, .. } => f
                .debug_struct("ModifyProvidedMana")
                .field("mana_diff", mana_diff)
                .finish(),
            Self::OverrideValidPlayZone { .. } => f.debug_struct("OverrideValidPlayZone").finish(),
            Self::ModifyManaCost {
                mana_diff, zones, ..
            } => f
                .debug_struct("ModifyManaCost")
                .field("mana_diff", mana_diff)
                .field("zones", zones)
                .finish(),
            Self::TriggeredEffect {
                trigger_on_effect, ..
            } => f
                .debug_struct("AddTriggeredEffect")
                .field("trigger_on_effect", trigger_on_effect)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Turn {
    pub(crate) player_id: PlayerId,
    pub(crate) controller_override: Option<PlayerId>,
}

impl Turn {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            player_id,
            controller_override: None,
        }
    }

    pub fn controlled_by(player_id: PlayerId, controller_id: PlayerId) -> Self {
        Self {
            player_id,
            controller_override: (player_id != controller_id).then_some(controller_id),
        }
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn controller_override(&self) -> Option<PlayerId> {
        self.controller_override
    }

    pub fn controller_id(&self) -> PlayerId {
        self.controller_override.unwrap_or(self.player_id)
    }
}

#[derive(Debug, Clone)]
pub struct TurnIterator {
    current: Turn,
    normal: Vec<Turn>,
    index: usize,
    overrides: VecDeque<Turn>,
}

impl TurnIterator {
    pub fn new(normal: Vec<PlayerId>) -> Self {
        assert!(!normal.is_empty());

        let normal: Vec<Turn> = normal.into_iter().map(Turn::new).collect();
        Self {
            current: normal[0].clone(),
            normal,
            index: 1,
            overrides: VecDeque::new(),
        }
    }

    pub fn current(&self) -> &Turn {
        &self.current
    }

    pub fn next_turn(&self) -> &Turn {
        self.overrides
            .front()
            .unwrap_or_else(|| &self.normal[self.index])
    }

    pub fn override_next(&mut self, value: Turn) {
        self.overrides.push_front(value);
    }

    pub fn override_upcoming<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Turn>,
    {
        self.overrides.extend(values);
    }

    pub fn skip_next_for(&mut self, player_id: &PlayerId) {
        if let Some(position) = self
            .overrides
            .iter()
            .position(|turn| turn.player_id() == *player_id)
        {
            self.overrides.remove(position);
            return;
        }

        let Some(position) = (0..self.normal.len()).find(|offset| {
            self.normal[(self.index + offset) % self.normal.len()].player_id() == *player_id
        }) else {
            return;
        };

        self.overrides.extend(
            (0..position)
                .map(|offset| self.normal[(self.index + offset) % self.normal.len()].clone()),
        );

        self.index = (self.index + position + 1) % self.normal.len();
    }
}

impl Iterator for TurnIterator {
    type Item = Turn;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.overrides.pop_front() {
            self.current = value;
            return Some(self.current.clone());
        }

        let value = self.normal[self.index].clone();
        self.index = (self.index + 1) % self.normal.len();
        self.current = value;
        Some(self.current.clone())
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub game_id: uuid::Uuid,
    pub players: Vec<Player>,
    pub turns: usize,
    pub cards: HashMap<CardId, Box<dyn Card>>,
    pub(crate) removed_cards: HashMap<CardId, Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub phase: Phase,
    pub curr_turn: TurnIterator,
    pub effects: EffectState,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
    pub ongoing_effects: Vec<TimedOngoingEffect>,
    pub player_mana: HashMap<PlayerId, u8>,
    pub eliminated_players: HashSet<PlayerId>,
    pub players_with_accepted_hands: HashSet<PlayerId>,
    next_ongoing_effect_timestamp: u64,
    runtime_cache: StateRuntimeCache,
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
        let mut player_ids = vec![];
        for player in players_with_decks {
            cards.extend(player.cards);
            decks.insert(player.player.id, player.deck);
            player_ids.push(player.player.id);
        }

        State {
            game_id,
            players,
            cards: cards.into_iter().map(|c| (*c.get_id(), c)).collect(),
            removed_cards: HashMap::new(),
            decks,
            turns: 0,
            phase: Phase::Mulligan,
            curr_turn: TurnIterator::new(player_ids),
            effects: EffectState::default(),
            player_one,
            server_tx,
            client_rx,
            ongoing_effects: Vec::new(),
            player_mana,
            eliminated_players: HashSet::new(),
            players_with_accepted_hands: HashSet::new(),
            next_ongoing_effect_timestamp: 1,
            runtime_cache: StateRuntimeCache::default(),
        }
    }

    /// validate_client_message checks that the message contains valid references to game entities
    /// (e.g. card ids). It should be called before processing a message from the client, and can be
    /// used to catch client bugs or malicious messages.
    pub(super) fn validate_client_message(&self, msg: &ClientMessage) -> anyhow::Result<()> {
        // Validate that the message is for the correct game.
        if self.game_id != msg.game_id() {
            return Err(anyhow::anyhow!(
                "message game id {} does not match state game id {}",
                msg.game_id(),
                self.game_id
            ));
        }

        // Validate that the player is in the game.
        if self
            .players
            .iter()
            .find(|p| &p.id == msg.player_id())
            .is_none()
        {
            return Err(anyhow::anyhow!(
                "message player id {} does not match any player in state",
                msg.player_id()
            ));
        }

        // Validate that all cards mentioned in the message exist in the game.
        match msg {
            ClientMessage::ClickCard { card_id, .. }
            | ClientMessage::RequestPlayableZones { card_id, .. }
            | ClientMessage::RequestAuraAffectedZones { card_id, .. }
            | ClientMessage::PlayCardAtZone { card_id, .. }
            | ClientMessage::PickCard { card_id, .. } => {
                self.cards
                    .get(card_id)
                    .ok_or(anyhow::anyhow!("invalid card id"))?;
                Ok(())
            }
            ClientMessage::PickCards { card_ids, .. } => {
                for card_id in card_ids {
                    self.cards
                        .get(card_id)
                        .ok_or(anyhow::anyhow!("invalid card id"))?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn find_caster(&self, spell_id: &CardId) -> Option<CardId> {
        self.effect_log().iter().find_map(|e| match e.effect {
            Effect::PlayMagic {
                card_id, caster_id, ..
            } if card_id == *spell_id => Some(caster_id),
            _ => None,
        })
    }

    pub fn current_turn(&self) -> &Turn {
        self.curr_turn.current()
    }

    pub fn current_player(&self) -> PlayerId {
        self.current_turn().player_id()
    }

    pub fn current_turn_controller(&self) -> PlayerId {
        self.current_turn().controller_id()
    }

    pub fn decision_player(&self, player_id: impl AsRef<PlayerId>) -> PlayerId {
        if player_id.as_ref() == &self.current_player() {
            self.current_turn_controller()
        } else {
            *player_id.as_ref()
        }
    }

    pub fn advance_turn(&mut self) -> Turn {
        self.curr_turn
            .next()
            .expect("turn iterator should produce turns forever")
    }

    pub fn advance_to_turn(&mut self, player_id: &PlayerId) -> anyhow::Result<Turn> {
        if &self.current_player() != player_id {
            let turn = self.advance_turn();
            if &turn.player_id() != player_id {
                return Err(anyhow::anyhow!(
                    "expected next turn for player {}, got {}",
                    player_id,
                    turn.player_id()
                ));
            }
        }

        Ok(self.current_turn().clone())
    }

    pub fn next_turn(&self) -> &Turn {
        self.curr_turn.next_turn()
    }

    pub fn override_next_turn(&mut self, turn: Turn) {
        self.curr_turn.override_next(turn);
    }

    pub fn skip_next_turn_for(&mut self, player_id: &PlayerId) {
        self.curr_turn.skip_next_for(player_id);
    }

    pub async fn replace_effect(&self, effect: &Effect) -> anyhow::Result<Option<Vec<Effect>>> {
        match effect {
            Effect::Attack {
                defender_id,
                attacker_id,
                ..
            } => {
                let defender = self.get_card(defender_id);
                if !self.is_unit_card(defender_id) {
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
                        from: attacker
                            .get_zone()
                            .clone()
                            .into_location()
                            .expect("Dodge Roll attacker must be in a location"),
                        to: LocationQuery::from_zone(defender.get_zone().clone()),
                        tap: true,
                        through_path: None,
                    },
                    Effect::PlayMagic {
                        player_id: defender_controller,
                        card_id: dodge_rolls_in_hand[0],
                        caster_id: avatar_id,
                        from: avatar
                            .get_zone()
                            .clone()
                            .into_location()
                            .expect("Dodge Roll caster must be in a location"),
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
        card_id: &CardId,
        target_zone: Option<&Zone>,
        player_id: &PlayerId,
    ) -> anyhow::Result<Costs> {
        let card = self.get_card(card_id);
        let base_costs = card.get_costs(self)?.clone();

        let total_mana_diff: i8 = self
            .active_continuous_effects()
            .into_iter()
            .filter_map(|ce| match ce {
                OngoingEffect::ModifyManaCost {
                    mana_diff,
                    affected_cards,
                    zones,
                } => {
                    if !affected_cards.matches(card.get_id(), self) {
                        return None;
                    }
                    let zone_ok = match zones {
                        None => true,
                        Some(effect_zones) => target_zone
                            .map(|z| effect_zones.options(self).contains(z))
                            .unwrap_or_default(),
                    };
                    if zone_ok { Some(mana_diff) } else { None }
                }
                _ => None,
            })
            .sum();

        let ignore_thresholds = self
            .temporary_effects()
            .iter()
            .find(|te| matches!(te, TemporaryEffect::IgnoreCostThresholds { affected_cards, for_player, .. } if affected_cards.matches(card.get_id(), self) && for_player == player_id))
            .is_some();

        let mut thresholds_diff = ThresholdsDiff::default();
        if ignore_thresholds {
            thresholds_diff = card.get_costs(self)?.thresholds_cost().into();
            thresholds_diff = thresholds_diff.negate();
        }

        Ok(base_costs
            .with_mana_adjusted(total_mana_diff)
            .with_thresholds_adjusted(thresholds_diff))
    }

    pub fn queue(&mut self, effects: impl IntoIterator<Item = Effect>) {
        self.invalidate_runtime_caches();
        self.effects.extend(effects);
    }

    pub fn queue_one(&mut self, effect: Effect) {
        self.invalidate_runtime_caches();
        self.effects.push_back(effect);
    }

    pub fn queue_front(&mut self, effect: Effect) {
        self.invalidate_runtime_caches();
        self.effects.push_front(effect);
    }

    pub fn effect_log(&self) -> &[LoggedEffect] {
        self.effects.log()
    }

    pub fn effect_log_mut(&mut self) -> &mut Vec<LoggedEffect> {
        self.invalidate_runtime_caches();
        self.effects.log_mut()
    }

    pub fn temporary_effects(&self) -> &[TemporaryEffect] {
        self.effects.temporary()
    }

    pub fn temporary_effects_mut(&mut self) -> &mut Vec<TemporaryEffect> {
        self.invalidate_runtime_caches();
        self.effects.temporary_mut()
    }

    pub fn animated_unit_base(&self, card_id: &CardId) -> Option<&UnitBase> {
        self.temporary_effects()
            .iter()
            .find_map(|effect| match effect {
                TemporaryEffect::Animate {
                    card_id: animated_id,
                    unit_base,
                    ..
                } if animated_id == card_id => Some(unit_base),
                _ => None,
            })
    }

    pub fn animated_unit_base_mut(&mut self, card_id: &CardId) -> Option<&mut UnitBase> {
        self.temporary_effects_mut()
            .iter_mut()
            .find_map(|effect| match effect {
                TemporaryEffect::Animate {
                    card_id: animated_id,
                    unit_base,
                    ..
                } if animated_id == card_id => Some(unit_base),
                _ => None,
            })
    }

    pub fn is_unit_card(&self, card_id: &CardId) -> bool {
        self.get_card(card_id).is_unit() || self.animated_unit_base(card_id).is_some()
    }

    pub fn is_minion_card(&self, card_id: &CardId) -> bool {
        self.is_unit_card(card_id) && !self.get_card(card_id).is_avatar()
    }

    pub fn deferred_effects(&self) -> &[DeferredEffect] {
        self.effects.deferred()
    }

    pub fn deferred_effects_mut(&mut self) -> &mut Vec<DeferredEffect> {
        self.invalidate_runtime_caches();
        self.effects.deferred_mut()
    }

    pub fn invalidate_runtime_caches(&self) {
        self.runtime_cache.clear();
    }

    pub async fn reconcile_ongoing_effects_for_test(&mut self) -> anyhow::Result<()> {
        self.invalidate_runtime_caches();

        let sources_to_remove = self
            .ongoing_effects
            .iter()
            .filter_map(|effect| {
                let source_id = effect.source?;
                let source_in_play = self
                    .cards
                    .get(&source_id)
                    .is_some_and(|card| card.get_zone().is_in_play());
                if !source_in_play {
                    Some(source_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        for source_id in sources_to_remove {
            self.remove_ongoing_effects_from_source(&source_id);
        }

        let in_play_sources = self
            .cards
            .values()
            .filter(|card| card.get_zone().is_in_play())
            .map(|card| *card.get_id())
            .collect::<Vec<_>>();
        for source_id in in_play_sources {
            let has_passive_effect = self
                .ongoing_effects
                .iter()
                .any(|effect| effect.source == Some(source_id));
            if !has_passive_effect {
                self.replace_passive_ongoing_effects_for_source(&source_id)
                    .await?;
            }
        }

        self.invalidate_runtime_caches();
        Ok(())
    }

    fn next_ongoing_effect_timestamp(&mut self) -> u64 {
        let timestamp = self.next_ongoing_effect_timestamp;
        self.next_ongoing_effect_timestamp += 1;
        timestamp
    }

    fn ongoing_effect_layer(effect: &TimedOngoingEffect) -> u8 {
        match &effect.effect {
            OngoingEffect::ControllerOverride { .. } => 3,
            OngoingEffect::ModifyPower { .. } | OngoingEffect::ModifyPowerForEach { .. } => 6,
            OngoingEffect::GrantStatus {
                status: CardStatus::Disabled | CardStatus::Silenced,
                ..
            } => 2,
            OngoingEffect::RemoveAbilities { .. } => 2,
            OngoingEffect::GrantAbility { .. }
            | OngoingEffect::GrantStatus { .. }
            | OngoingEffect::GrantActivatedAbility { .. }
            | OngoingEffect::GrantCounter { .. } => 5,
            _ => 5,
        }
    }

    fn ordered_ongoing_effects(&self) -> Vec<&TimedOngoingEffect> {
        let mut effects = self.ongoing_effects.iter().enumerate().collect::<Vec<_>>();
        effects.sort_by_key(|(idx, effect)| {
            (Self::ongoing_effect_layer(effect), effect.timestamp, *idx)
        });
        effects.into_iter().map(|(_, effect)| effect).collect()
    }

    pub fn active_continuous_effects(&self) -> Vec<&OngoingEffect> {
        let mut inactive_sources = HashSet::new();
        self.ordered_ongoing_effects()
            .into_iter()
            .filter_map(|timed_effect| {
                let active = timed_effect
                    .source
                    .is_none_or(|source_id| !inactive_sources.contains(&source_id));

                if active {
                    match &timed_effect.effect {
                        OngoingEffect::GrantStatus {
                            status: CardStatus::Disabled | CardStatus::Silenced,
                            affected_cards,
                        } => {
                            inactive_sources.extend(affected_cards.all(self));
                        }
                        OngoingEffect::RemoveAbilities {
                            removal,
                            affected_cards,
                        } if removal.removes_special_abilities() => {
                            inactive_sources.extend(affected_cards.all(self));
                        }
                        _ => {}
                    }
                    Some(&timed_effect.effect)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn ongoing_effects_data(&self) -> Vec<OngoingEffectData> {
        let mut inactive_sources = HashSet::new();
        self.ordered_ongoing_effects()
            .into_iter()
            .map(|timed_effect| {
                let active = timed_effect
                    .source
                    .is_none_or(|source_id| !inactive_sources.contains(&source_id));

                let mut affected_card_ids = timed_effect.effect.affected_card_ids(self);
                affected_card_ids.sort();
                affected_card_ids.dedup();

                let mut affected_zones = timed_effect.effect.explicit_affected_zones(self);
                affected_zones.extend(affected_card_ids.iter().filter_map(|card_id| {
                    self.cards.get(card_id).and_then(|card| {
                        let zone = card.get_zone().clone();
                        zone.is_in_play().then_some(zone)
                    })
                }));
                affected_zones.sort();
                affected_zones.dedup();

                if active
                    && let OngoingEffect::GrantStatus {
                        status: CardStatus::Disabled | CardStatus::Silenced,
                        affected_cards,
                    } = &timed_effect.effect
                {
                    inactive_sources.extend(affected_cards.all(self));
                }

                let source_name = timed_effect
                    .source
                    .and_then(|source_id| self.cards.get(&source_id))
                    .map(|card| card.get_name().to_string());

                OngoingEffectData {
                    source_card_id: timed_effect.source,
                    source_name,
                    description: timed_effect.effect.display_description(),
                    timestamp: timed_effect.timestamp,
                    active,
                    affected_card_ids,
                    affected_zones,
                }
            })
            .collect()
    }

    pub fn remove_ongoing_effects_from_source(&mut self, source_id: &uuid::Uuid) {
        self.ongoing_effects
            .retain(|effect| effect.source != Some(*source_id));
        self.invalidate_runtime_caches();
    }

    async fn replace_passive_ongoing_effects_for_source(
        &mut self,
        source_id: &uuid::Uuid,
    ) -> anyhow::Result<()> {
        let timestamp = match self
            .ongoing_effects
            .iter()
            .find(|effect| effect.source == Some(*source_id))
            .map(|effect| effect.timestamp)
        {
            Some(timestamp) => timestamp,
            None => self.next_ongoing_effect_timestamp(),
        };

        let Some(card) = self.cards.get(source_id) else {
            self.remove_ongoing_effects_from_source(source_id);
            return Ok(());
        };
        if !card.get_zone().is_in_play() {
            self.remove_ongoing_effects_from_source(source_id);
            return Ok(());
        }

        let mut passive_effects = card.get_continuous_effects(self).await?;
        passive_effects.extend(card.area_modifiers(self));

        self.ongoing_effects
            .retain(|effect| effect.source != Some(*source_id));
        for effect in passive_effects {
            self.ongoing_effects.push(TimedOngoingEffect {
                effect,
                source: Some(*source_id),
                timestamp,
            });
        }
        self.invalidate_runtime_caches();
        Ok(())
    }

    pub async fn add_passive_ongoing_effects_for_source(
        &mut self,
        source_id: &uuid::Uuid,
    ) -> anyhow::Result<()> {
        self.replace_passive_ongoing_effects_for_source(source_id)
            .await?;

        let existing_sources = self
            .ongoing_effects
            .iter()
            .filter_map(|effect| {
                let existing_source = effect.source?;
                if existing_source != *source_id {
                    Some(existing_source)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        for existing_source in existing_sources {
            self.replace_passive_ongoing_effects_for_source(&existing_source)
                .await?;
        }

        Ok(())
    }

    fn with_area_modifier_index<T>(&self, f: impl FnOnce(&AreaModifierIndex) -> T) -> T {
        {
            let index = self
                .runtime_cache
                .area_modifier_index
                .read()
                .expect("area modifier index lock should not be poisoned");
            if let Some(index) = index.as_ref() {
                return f(index);
            }
        }

        {
            let index = AreaModifierIndex::build(self);
            let mut cached = self
                .runtime_cache
                .area_modifier_index
                .write()
                .expect("area modifier index lock should not be poisoned");
            if cached.is_none() {
                *cached = Some(index);
            }
        }

        let index = self
            .runtime_cache
            .area_modifier_index
            .read()
            .expect("area modifier index lock should not be poisoned");
        f(index
            .as_ref()
            .expect("area modifier index should be initialized"))
    }

    fn with_continuous_effect_index<T>(&self, f: impl FnOnce(&OngoingEffectIndex) -> T) -> T {
        {
            let index = self
                .runtime_cache
                .continuous_effect_index
                .read()
                .expect("continuous effect index lock should not be poisoned");
            if let Some(index) = index.as_ref() {
                return f(index);
            }
        }

        {
            let index = OngoingEffectIndex::build(self);
            let mut cached = self
                .runtime_cache
                .continuous_effect_index
                .write()
                .expect("continuous effect index lock should not be poisoned");
            if cached.is_none() {
                *cached = Some(index);
            }
        }

        let index = self
            .runtime_cache
            .continuous_effect_index
            .read()
            .expect("continuous effect index lock should not be poisoned");
        f(index
            .as_ref()
            .expect("continuous effect index should be initialized"))
    }

    pub fn ability_modifiers_from_area_modifiers(&self, card_id: &CardId) -> Vec<AbilityModifier> {
        self.with_area_modifier_index(|index| {
            index
                .ability_modifiers
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn card_has_special_abilities_removed(&self, card_id: &CardId) -> bool {
        self.get_card(card_id)
            .has_status(self, &CardStatus::Silenced)
            || self
                .get_card(card_id)
                .has_status(self, &CardStatus::Disabled)
    }

    pub fn counters_from_area_modifiers(&self, card_id: &CardId) -> Vec<Counter> {
        self.with_area_modifier_index(|index| {
            index
                .grants_counters
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn activated_abilities_from_area_modifiers(
        &self,
        card_id: &CardId,
    ) -> Vec<Box<dyn ActivatedAbility>> {
        self.with_area_modifier_index(|index| {
            index
                .grants_activated_abilities
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn granted_statuses_from_continuous_effects(&self, card_id: &CardId) -> Vec<CardStatus> {
        self.with_continuous_effect_index(|index| {
            index
                .grants_statuses
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn activated_abilities_from_continuous_effects(
        &self,
        card_id: &CardId,
    ) -> Vec<Box<dyn ActivatedAbility>> {
        self.with_continuous_effect_index(|index| {
            index
                .grants_activated_abilities
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn power_diff_from_continuous_effects(&self, card_id: &CardId) -> i16 {
        self.with_continuous_effect_index(|index| {
            index.power_diffs.get(card_id).copied().unwrap_or_default()
        })
    }

    pub fn get_thresholds_for_player(&self, player_id: &PlayerId) -> Thresholds {
        self.cards
            .values()
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
            .values()
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

    pub fn get_spans_of_land(&self) -> Vec<Vec<Zone>> {
        let mut visited: Vec<Zone> = Vec::new();
        let mut spans_of_land: Vec<Vec<Zone>> = Vec::new();

        fn dfs(state: &State, zone: &Zone, visited: &mut Vec<Zone>, span: &mut Vec<Zone>) {
            if visited.iter().any(|z| z == zone) {
                return;
            }
            visited.push(zone.clone());

            if let Some(site) = zone.get_site(state) {
                let is_land = site.provided_affinity(state).unwrap_or_default().water == 0;
                if is_land {
                    if !span.iter().any(|z| z == zone) {
                        span.push(zone.clone());
                    }
                    for adj in zone.get_adjacent() {
                        dfs(state, &adj, visited, span);
                    }
                }
            }
        }

        for card in self
            .cards
            .values()
            .filter(|c| c.get_card_type() == CardType::Site)
        {
            let zone = card.get_zone();
            if !zone.is_in_play() {
                continue;
            }

            if let Some(site) = zone.get_site(self) {
                let is_land = site.provided_affinity(self).unwrap_or_default().water == 0;
                if is_land && !visited.iter().any(|z| z == zone) {
                    let mut span: Vec<Zone> = Vec::new();
                    dfs(self, zone, &mut visited, &mut span);
                    if !span.is_empty() {
                        spans_of_land.push(span);
                    }
                }
            }
        }

        spans_of_land
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

    pub fn eliminate_player(&mut self, player_id: PlayerId) {
        self.eliminated_players.insert(player_id);
    }

    pub fn is_player_eliminated(&self, player_id: &PlayerId) -> bool {
        self.eliminated_players.contains(player_id)
    }

    pub fn living_players(&self) -> Vec<&Player> {
        self.players
            .iter()
            .filter(|player| !self.is_player_eliminated(&player.id))
            .collect()
    }

    pub fn winner_if_game_over(&self) -> Option<&Player> {
        let living_players = self.living_players();
        (living_players.len() == 1).then_some(living_players[0])
    }

    pub fn get_defenders_for_attack(
        &self,
        attacker_id: &CardId,
        defender_id: &CardId,
    ) -> Vec<CardId> {
        let defender = self.get_card(defender_id);
        let controller_id = defender.get_controller_id(self);
        let mut defenders = CardQuery::new()
            .units()
            .near_to(defender.get_zone())
            .without_ability(&Ability::CannotDefend)
            .without_status(&CardStatus::Disabled)
            .untapped()
            .id_not(defender_id)
            .controlled_by(&controller_id)
            .all(self);

        let extra_defenders: Vec<CardId> = self
            .cards
            .values()
            .filter(|card| card.get_controller_id(self) == controller_id)
            .filter(|card| !defenders.contains(card.get_id()))
            .filter(|card| card.can_defend_attack(self, attacker_id, defender_id))
            .map(|card| *card.get_id())
            .collect();
        defenders.extend(extra_defenders);
        defenders
    }

    pub fn get_interceptors_for_move(
        &self,
        path: &[Zone],
        moving_card_id: &CardId,
        controller_id: &PlayerId,
    ) -> Vec<CardId> {
        let mut interceptors = Vec::new();
        let Some(final_zone) = path.last() else {
            return interceptors;
        };

        let moving_card = self.get_card(moving_card_id);
        if moving_card.has_ability(self, &Ability::Stealth)
            || moving_card.has_ability(self, &Ability::Uninterceptable)
        {
            return interceptors;
        }
        let moving_card_is_airborne = moving_card.has_ability(self, &Ability::Airborne);

        for card in self.cards.values() {
            if &card.get_controller_id(self) != controller_id {
                continue;
            }
            if !self.is_unit_card(card.get_id()) {
                continue;
            }
            if !card.get_zone().is_in_play() {
                continue;
            }
            if card.has_status(self, &CardStatus::Disabled) {
                continue;
            }
            if card.is_tapped() {
                continue;
            }
            if !card.occupies_zone(self, final_zone) {
                continue;
            }
            if moving_card_is_airborne
                && !card.has_ability(self, &Ability::Airborne)
                && !card.is_ranged(self).unwrap_or(false)
            {
                continue;
            }

            interceptors.push(*card.get_id());
        }

        interceptors
    }

    pub async fn apply_effects_without_log(&mut self) -> anyhow::Result<()> {
        EffectEngine::drain_without_log(self).await
    }

    pub fn data_from_cards(&self) -> Vec<CardData> {
        self.cards
            .values()
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
                statuses: c.get_statuses(self),
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
                    .saturating_sub(avatar_card.get_damage_taken().unwrap_or(0)),
            );
        }

        Ok(ServerMessage::Sync {
            cards: self.data_from_cards(),
            resources: self
                .players
                .iter()
                .map(|p| (p.id, self.get_player_resources(&p.id).unwrap().clone()))
                .collect(),
            current_player: self.current_turn_controller(),
            turn_player: self.current_player(),
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

    pub fn get_card_mut(&mut self, card_id: &CardId) -> &mut dyn Card {
        self.invalidate_runtime_caches();
        &mut **self.cards.get_mut(card_id).expect("card to exist")
    }

    pub fn get_card(&self, card_id: &CardId) -> &dyn Card {
        self.cards
            .get(card_id)
            .or_else(|| self.removed_cards.get(card_id))
            .map(|card| &**card)
            .expect("card to exist")
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

    #[cfg(any(test, feature = "benchmark"))]
    pub fn new_mock_state(zones_with_sites: impl AsRef<[u8]>) -> State {
        use crate::card::{AridDesert, Sorcerer, from_name_and_zone};
        use crate::zone::Location;

        let player_one_id = uuid::Uuid::new_v4();
        let player_two_id = uuid::Uuid::new_v4();
        let avatar_one = Sorcerer::new(player_one_id);
        let avatar_one_id = *avatar_one.get_id();
        let avatar_two = Sorcerer::new(player_two_id);
        let avatar_two_id = *avatar_two.get_id();
        let mut cards: Vec<Box<dyn Card>> = zones_with_sites
            .as_ref()
            .iter()
            .map(|z| {
                use crate::card::Region;

                from_name_and_zone(
                    AridDesert::NAME,
                    &player_one_id,
                    Zone::Location(Location::Square(*z, Region::Surface)),
                )
            })
            .collect();
        cards.push(Box::new(avatar_one));
        cards.push(Box::new(avatar_two));

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
                avatar_one_id,
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
                avatar_two_id,
            ),
            cards: vec![],
        };

        let players = vec![player1, player2];
        let (server_tx, _) = async_channel::unbounded();
        let (_, client_rx) = async_channel::unbounded();
        State::new(uuid::Uuid::new_v4(), players, server_tx, client_rx)
    }
}
