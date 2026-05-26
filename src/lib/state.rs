use crate::{
    card::{Ability, Card, CardData, CardType, Costs, DodgeRoll, SiteType},
    deck::Deck,
    effect::Counter,
    effect::{Effect, EffectCallback, EffectEngine, EffectState},
    game::{
        ActivatedAbility, InputStatus, PlayerId, Resources, Thresholds, ThresholdsDiff, pick_zone,
        yes_or_no,
    },
    networking::message::{ClientMessage, ServerMessage},
    query::{CardQuery, EffectQuery, ZoneQuery},
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
    pub ability_modifiers: HashMap<uuid::Uuid, Vec<AbilityModifier>>,
    pub grants_activated_abilities: HashMap<uuid::Uuid, Vec<Box<dyn ActivatedAbility>>>,
    pub grants_counters: HashMap<uuid::Uuid, Vec<Counter>>,
}

#[derive(Debug, Clone)]
pub enum AbilityModifier {
    Grant(Ability),
    Remove(Ability),
}

#[derive(Debug, Clone)]
pub struct TimedOngoingEffect {
    pub effect: OngoingEffect,
    pub source: Option<uuid::Uuid>,
    pub timestamp: u64,
}

#[derive(Debug, Default, Clone)]
pub struct ContinuousEffectIndex {
    pub grants_abilities: HashMap<uuid::Uuid, Vec<Ability>>,
    pub grants_activated_abilities: HashMap<uuid::Uuid, Vec<Box<dyn ActivatedAbility>>>,
    pub power_diffs: HashMap<uuid::Uuid, i16>,
}

impl ContinuousEffectIndex {
    fn build(state: &State) -> Self {
        let mut index = Self::default();
        let mut blocked_sources = HashSet::new();

        for effect in state.ordered_ongoing_effects() {
            if effect
                .source
                .is_some_and(|source_id| blocked_sources.contains(&source_id))
            {
                continue;
            }

            match &effect.effect {
                ContinuousEffect::GrantAbility {
                    ability,
                    affected_cards,
                } => {
                    if ability == &Ability::Disabled {
                        for card_id in affected_cards.all(state) {
                            blocked_sources.insert(card_id);
                        }
                    }
                }
                ContinuousEffect::RemoveAbilities {
                    abilities,
                    affected_cards,
                } => {
                    if is_silence_modifier(abilities) {
                        for card_id in affected_cards.all(state) {
                            blocked_sources.insert(card_id);
                        }
                    }
                }
                ContinuousEffect::ModifyPower {
                    power_diff,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        *index.power_diffs.entry(card_id).or_default() += *power_diff;
                    }
                }
                ContinuousEffect::ModifyPowerForEach {
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
        let mut blocked_sources = HashSet::new();

        for effect in state.ordered_ongoing_effects() {
            if effect
                .source
                .is_some_and(|source_id| blocked_sources.contains(&source_id))
            {
                continue;
            }

            match &effect.effect {
                ContinuousEffect::GrantAbility {
                    ability,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        index
                            .ability_modifiers
                            .entry(card_id)
                            .or_default()
                            .push(AbilityModifier::Grant(ability.clone()));
                        if ability == &Ability::Disabled {
                            blocked_sources.insert(card_id);
                        }
                    }
                }
                ContinuousEffect::RemoveAbilities {
                    abilities,
                    affected_cards,
                } => {
                    let affected_cards = affected_cards.all(state);
                    if is_silence_modifier(abilities) {
                        blocked_sources.extend(affected_cards.iter().copied());
                    }
                    for card_id in affected_cards {
                        index
                            .ability_modifiers
                            .entry(card_id)
                            .or_default()
                            .extend(abilities.iter().cloned().map(AbilityModifier::Remove));
                    }
                }
                ContinuousEffect::GrantActivatedAbility {
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
                ContinuousEffect::GrantCounter {
                    counter,
                    affected_cards,
                } => {
                    for card_id in affected_cards.all(state) {
                        if !state.get_card(&card_id).is_flooded_site(state) {
                            index
                                .grants_counters
                                .entry(card_id)
                                .or_default()
                                .push(counter.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        index
    }
}

fn is_silence_modifier(abilities: &[Ability]) -> bool {
    let silenced = crate::card::silenced_abilities();
    silenced.iter().all(|ability| abilities.contains(ability))
}

#[derive(Debug, Default)]
pub struct StateRuntimeCache {
    area_modifier_index: RwLock<Option<AreaModifierIndex>>,
    continuous_effect_index: RwLock<Option<ContinuousEffectIndex>>,
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
    FloodSites {
        affected_sites: CardQuery,
    },
    DroughtSites {
        affected_sites: CardQuery,
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
    RemoveAbilities {
        abilities: Vec<Ability>,
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
        new_affinities: Thresholds,
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

pub type ContinuousEffect = OngoingEffect;

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
            Self::FloodSites { .. } => f.debug_struct("FloodSites").finish(),
            Self::DroughtSites { .. } => f.debug_struct("DroughtSites").finish(),
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
            Self::RemoveAbilities { abilities, .. } => f
                .debug_struct("RemoveAbilities")
                .field("abilities", abilities)
                .finish(),
            Self::GrantActivatedAbility { .. } => f.debug_struct("GrantActivatedAbility").finish(),
            Self::GrantCounter { counter, .. } => f
                .debug_struct("GrantCounter")
                .field("counter", counter)
                .finish(),
            Self::ModifyProvidedAffinities { new_affinities, .. } => f
                .debug_struct("ModifyProvidedAffinities")
                .field("new_affinities", new_affinities)
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
    pub cards: HashMap<uuid::Uuid, Box<dyn Card>>,
    pub decks: HashMap<PlayerId, Deck>,
    pub input_status: InputStatus,
    pub phase: Phase,
    pub waiting_for_input: bool,
    pub curr_turn: TurnIterator,
    pub effects: EffectState,
    pub player_one: PlayerId,
    pub server_tx: Sender<ServerMessage>,
    pub client_rx: Receiver<ClientMessage>,
    pub continuous_effects: Vec<ContinuousEffect>,
    pub ongoing_effects: Vec<TimedOngoingEffect>,
    pub player_mana: HashMap<PlayerId, u8>,
    pub loosers: HashSet<PlayerId>,
    pub players_skipping_turns: HashSet<PlayerId>,
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
            decks,
            turns: 0,
            input_status: InputStatus::None,
            phase: Phase::Mulligan,
            curr_turn: TurnIterator::new(player_ids),
            waiting_for_input: false,
            effects: EffectState::default(),
            player_one,
            server_tx,
            client_rx,
            continuous_effects: Vec::new(),
            ongoing_effects: Vec::new(),
            player_mana,
            loosers: HashSet::new(),
            players_skipping_turns: HashSet::new(),
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

    pub fn find_caster(&self, spell_id: &uuid::Uuid) -> Option<uuid::Uuid> {
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

    pub async fn replace_effect(&self, effect: &Effect) -> anyhow::Result<Option<Vec<Effect>>> {
        match effect {
            Effect::Attack {
                defender_id,
                attacker_id,
                ..
            } => {
                let defender = self.get_card(defender_id);
                if !defender.is_unit() {
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
                        from: attacker.get_zone().clone(),
                        to: ZoneQuery::from_zone(defender.get_zone().clone()),
                        tap: true,
                        region: attacker.get_region(self).clone(),
                        through_path: None,
                    },
                    Effect::PlayMagic {
                        player_id: defender_controller,
                        card_id: dodge_rolls_in_hand[0],
                        caster_id: avatar_id,
                        from: avatar.get_zone().clone(),
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
        card_id: &uuid::Uuid,
        target_zone: Option<&Zone>,
        player_id: &PlayerId,
    ) -> anyhow::Result<Costs> {
        let card = self.get_card(card_id);
        let base_costs = card.get_costs(self)?.clone();

        let total_mana_diff: i8 = self
            .continuous_effects
            .iter()
            .filter_map(|ce| match ce {
                ContinuousEffect::ModifyManaCost {
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
                    if zone_ok { Some(*mana_diff) } else { None }
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

        self.refresh_continuous_effects();
        Ok(())
    }

    fn next_ongoing_effect_timestamp(&mut self) -> u64 {
        let timestamp = self.next_ongoing_effect_timestamp;
        self.next_ongoing_effect_timestamp += 1;
        timestamp
    }

    fn ongoing_effect_layer(effect: &TimedOngoingEffect) -> u8 {
        match &effect.effect {
            ContinuousEffect::ControllerOverride { .. } => 3,
            ContinuousEffect::ModifyPower { .. } | ContinuousEffect::ModifyPowerForEach { .. } => 6,
            ContinuousEffect::GrantAbility {
                ability: Ability::Disabled,
                ..
            } => 2,
            ContinuousEffect::RemoveAbilities { .. } => 2,
            ContinuousEffect::FloodSites { .. }
            | ContinuousEffect::DroughtSites { .. }
            | ContinuousEffect::GrantAbility { .. }
            | ContinuousEffect::GrantActivatedAbility { .. }
            | ContinuousEffect::GrantCounter { .. } => 5,
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

    fn refresh_continuous_effects(&mut self) {
        self.continuous_effects = self
            .ordered_ongoing_effects()
            .into_iter()
            .map(|effect| effect.effect.clone())
            .collect();
        self.invalidate_runtime_caches();
    }

    pub fn remove_ongoing_effects_from_source(&mut self, source_id: &uuid::Uuid) {
        self.ongoing_effects
            .retain(|effect| effect.source != Some(*source_id));
        self.refresh_continuous_effects();
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
        self.refresh_continuous_effects();
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

    fn with_continuous_effect_index<T>(&self, f: impl FnOnce(&ContinuousEffectIndex) -> T) -> T {
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
            let index = ContinuousEffectIndex::build(self);
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

    pub fn ability_modifiers_from_area_modifiers(
        &self,
        card_id: &uuid::Uuid,
    ) -> Vec<AbilityModifier> {
        self.with_area_modifier_index(|index| {
            index
                .ability_modifiers
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn counters_from_area_modifiers(&self, card_id: &uuid::Uuid) -> Vec<Counter> {
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
        card_id: &uuid::Uuid,
    ) -> Vec<Box<dyn ActivatedAbility>> {
        self.with_area_modifier_index(|index| {
            index
                .grants_activated_abilities
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn granted_abilities_from_continuous_effects(&self, card_id: &uuid::Uuid) -> Vec<Ability> {
        self.with_continuous_effect_index(|index| {
            index
                .grants_abilities
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn activated_abilities_from_continuous_effects(
        &self,
        card_id: &uuid::Uuid,
    ) -> Vec<Box<dyn ActivatedAbility>> {
        self.with_continuous_effect_index(|index| {
            index
                .grants_activated_abilities
                .get(card_id)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn power_diff_from_continuous_effects(&self, card_id: &uuid::Uuid) -> i16 {
        self.with_continuous_effect_index(|index| {
            index.power_diffs.get(card_id).copied().unwrap_or_default()
        })
    }

    pub fn water_site_status_from_continuous_effects(&self, card_id: &uuid::Uuid) -> Option<bool> {
        let mut status = None;
        for effect in &self.continuous_effects {
            match effect {
                ContinuousEffect::FloodSites { affected_sites }
                    if affected_sites.matches(card_id, self) =>
                {
                    status = Some(true);
                }
                ContinuousEffect::DroughtSites { affected_sites }
                    if affected_sites.matches(card_id, self) =>
                {
                    status = Some(false);
                }
                _ => {}
            }
        }
        status
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

    pub fn get_defenders_for_attack(&self, defender_id: &uuid::Uuid) -> Vec<uuid::Uuid> {
        let defender = self.get_card(defender_id);
        CardQuery::new()
            .units()
            .near_to(defender.get_zone())
            .without_ability(&Ability::CannotDefend)
            .without_ability(&Ability::Disabled)
            .untapped()
            .id_not(defender_id)
            .controlled_by(&defender.get_controller_id(self))
            .all(self)
    }

    pub fn get_interceptors_for_move(
        &self,
        path: &[Zone],
        moving_card_id: &uuid::Uuid,
        controller_id: &PlayerId,
    ) -> Vec<uuid::Uuid> {
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
            if !card.is_unit() {
                continue;
            }
            if !card.get_zone().is_in_play() {
                continue;
            }
            if card.has_ability(self, &Ability::Disabled) {
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

    pub fn get_card_mut(&mut self, card_id: &uuid::Uuid) -> &mut dyn Card {
        self.invalidate_runtime_caches();
        &mut **self.cards.get_mut(card_id).expect("card to exist")
    }

    pub fn get_card(&self, card_id: &uuid::Uuid) -> &dyn Card {
        &**self.cards.get(card_id).expect("card to exist")
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
                    Zone::Location(*z, Region::Surface),
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
