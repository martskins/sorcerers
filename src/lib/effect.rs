use crate::zone::{Location, Zone};
use crate::{
    card::{
        Ability, Card, CardBaseMethods, CardStatus, Cost, Damage, FootSoldier, Frog, Region,
        Rubble, UnitBase,
    },
    game::{
        BaseAction, CardId, Direction, PlayerAction, PlayerId, SoundEffect, pick_card, pick_option,
    },
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, LocationQuery, QueryCache, ZoneQuery},
    state::{OngoingEffect, Phase, State, Turn},
};
use std::{collections::HashMap, fmt::Debug};

pub mod lifecycle;
pub mod log;
pub mod runtime;

pub use lifecycle::{
    DeferredEffect, EffectCallback, EffectLifecycle, EffectReplacementCallback, EffectState,
    TemporaryEffect,
};
pub use log::{EffectLogEmitter, LoggedEffect};
pub use runtime::EffectEngine;

fn can_use_special_abilities(state: &State, card_id: &CardId) -> bool {
    !state.card_has_special_abilities_removed(card_id)
}

fn location_survival_effects_for_cards(
    state: &State,
    card_ids: impl IntoIterator<Item = CardId>,
) -> Vec<Effect> {
    card_ids
        .into_iter()
        .filter_map(|card_id| state.get_card(&card_id).location_survival_effect(state))
        .collect()
}

fn location_survival_effects_for_zones(
    state: &State,
    zones: impl IntoIterator<Item = Zone>,
) -> Vec<Effect> {
    let mut squares = zones
        .into_iter()
        .flat_map(|zone| zone.squares())
        .collect::<Vec<_>>();
    squares.sort();
    squares.dedup();

    let affected_card_ids = state
        .cards
        .values()
        .filter(|card| card.get_zone().is_in_play())
        .filter(|card| {
            card.get_zone()
                .squares()
                .into_iter()
                .any(|square| squares.contains(&square))
        })
        .map(|card| *card.get_id())
        .collect::<Vec<_>>();

    location_survival_effects_for_cards(state, affected_card_ids)
}

fn location_survival_effects_for_realm(state: &State) -> Vec<Effect> {
    let card_ids = state
        .cards
        .values()
        .filter(|card| card.get_zone().is_in_play())
        .map(|card| *card.get_id())
        .collect::<Vec<_>>();

    location_survival_effects_for_cards(state, card_ids)
}

fn mana_effect_for_resource_entering_realm(
    state: &State,
    card_id: &CardId,
) -> anyhow::Result<Option<Effect>> {
    let card = state.get_card(card_id);
    let controller_id = card.get_controller_id(state);
    if controller_id != state.current_player() {
        return Ok(None);
    }

    let Some(resource_provider) = card.get_resource_provider() else {
        return Ok(None);
    };
    let mana = resource_provider.provided_mana(state)?;
    if mana == 0 {
        return Ok(None);
    }

    Ok(Some(Effect::AdjustMana {
        player_id: controller_id,
        mana: mana as i8,
    }))
}

#[derive(Debug, Clone)]
pub struct AbilityCounter {
    pub id: uuid::Uuid,
    pub ability: Ability,
    pub expires_on_effect: Option<EffectQuery>,
}

#[derive(Debug, Clone)]
pub struct StatusCounter {
    pub id: uuid::Uuid,
    pub status: CardStatus,
    pub expires_on_effect: Option<EffectQuery>,
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub id: uuid::Uuid,
    pub power: i16,
    pub toughness: i16,
    pub expires_on_effect: Option<EffectQuery>,
}

impl Counter {
    pub fn new(power: i16, toughness: i16, expires_on_effect: Option<EffectQuery>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            power,
            toughness,
            expires_on_effect,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TokenType {
    Rubble,
    FootSoldier,
    Frog,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum DrawKind {
    Site,
    Spell,
    Choice,
}

#[derive(Debug, Clone)]
pub enum Effect {
    Noop,
    PlayerLost {
        player_id: PlayerId,
    },
    SkipNextTurn {
        player_id: PlayerId,
    },
    OverrideNextTurn {
        turn: Turn,
    },
    SetAvatarLife {
        player_id: PlayerId,
        life: u16,
    },
    AdjustAvatarLife {
        player_id: PlayerId,
        amount: i16,
    },
    SummonToken {
        player_id: PlayerId,
        token_type: TokenType,
        zone: Zone,
    },
    Heal {
        card_id: CardId,
        amount: u16,
    },
    ShootProjectile {
        id: uuid::Uuid,
        range: Option<u8>,
        player_id: PlayerId,
        shooter: CardId,
        from_zone: Zone,
        direction: Direction,
        damage: u16,
        ranged_strike: bool,
        piercing: bool,
        splash_damage: Option<u16>,
    },
    RemoveAbility {
        card_id: CardId,
        modifier: Ability,
    },
    RemoveStatus {
        card_id: CardId,
        status: CardStatus,
    },
    AddAbilityCounter {
        card_id: CardId,
        counter: AbilityCounter,
    },
    AddStatusCounter {
        card_id: CardId,
        counter: StatusCounter,
    },
    // RemoveCardFromGame completely removes a card from the game, removing it from all zones and
    // clearing all references to it. This is primarily used for token cards, as when they leave the
    // board they hit the cemetery and then immediately cease to exist.
    RemoveCardFromGame {
        card_id: CardId,
    },
    AddCounter {
        card_id: CardId,
        counter: Counter,
    },
    SetCardRegion {
        card_id: CardId,
        destination: Region,
        tap: bool,
    },
    SetCardZone {
        card_id: CardId,
        zone: Zone,
    },
    DiscardCard {
        player_id: PlayerId,
        card_id: CardId,
    },
    MoveCard {
        player_id: PlayerId,
        card_id: CardId,
        from: Location,
        to: LocationQuery,
        tap: bool,
        through_path: Option<Vec<Zone>>,
    },
    DrawCard {
        player_id: PlayerId,
        count: u8,
        kind: DrawKind,
    },
    PlayMagic {
        player_id: PlayerId,
        card_id: CardId,
        caster_id: CardId,
        from: Location,
    },
    PlayCard {
        player_id: PlayerId,
        card_id: CardId,
        zone: ZoneQuery,
        spellcaster: CardId,
    },
    SummonCards {
        cards: Vec<(PlayerId, CardId, Zone, Location)>,
    },
    SetTapped {
        card_id: CardId,
        tapped: bool,
    },
    EndTurn {
        player_id: PlayerId,
    },
    FinishEndTurn {
        player_id: PlayerId,
    },
    StartTurn {
        player_id: PlayerId,
    },
    AdjustMana {
        player_id: PlayerId,
        mana: i8,
    },
    Strike {
        striker_id: CardId,
        target_id: CardId,
    },
    Attack {
        attacker_id: CardId,
        defender_id: CardId,
        defending_ids: Vec<CardId>,
        damage_assignment: Option<HashMap<CardId, u16>>,
    },
    TakeDamage {
        card_id: CardId,
        from: CardId,
        damage: Damage,
    },
    BanishCard {
        card_id: CardId,
    },
    KillMinion {
        card_id: CardId,
        killer_id: CardId,
        from_attack: bool,
    },
    BuryCard {
        card_id: CardId,
    },
    Animate {
        card_id: CardId,
        unit_base: UnitBase,
        expires_on_effect: EffectQuery,
    },
    SetCardData {
        card_id: CardId,
        data: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    },
    TeleportCard {
        player_id: PlayerId,
        card_id: CardId,
        to_location: Location,
    },
    RearrangeDeck {
        spells: Vec<CardId>,
        sites: Vec<CardId>,
    },
    AddDeferredEffect {
        effect: DeferredEffect,
    },
    AddTemporaryEffect {
        effect: TemporaryEffect,
    },
    SetBearer {
        card_id: CardId,
        bearer_id: Option<CardId>,
    },
    ShuffleDeck {
        player_id: PlayerId,
    },
    SetController {
        card_id: CardId,
        player_id: PlayerId,
    },
    MakeCardCopyOf {
        card_id: CardId,
        copy_source_id: CardId,
    },
    CopyMagic {
        source_id: CardId,
        player_id: PlayerId,
        card_id: CardId,
        caster_id: CardId,
    },
    CopyArtifact {
        player_id: PlayerId,
        artifact_id: CardId,
        bearer_id: Option<CardId>,
        caster_id: CardId,
    },
    /// Creates a token copy of the named card for the given player and summons it in the target
    /// zone. The copy triggers its Genesis, then is automatically banished afterwards.
    // TODO: This effect shouldn't banish it. We should queue a second effect in Deathspeaker to
    // banish it, so we can reuse this effect in other cards.
    SummonCopy {
        card_name: String,
        player_id: PlayerId,
        zone: Zone,
    },
}

fn player_name<'a>(player_id: &PlayerId, state: &'a State) -> &'a str {
    match state
        .players
        .iter()
        .enumerate()
        .find(|(_, p)| &p.id == player_id)
    {
        Some((idx, player)) if player.name.trim().is_empty() && idx == 0 => "Player 1",
        Some((_, player)) if player.name.trim().is_empty() => "Player 2",
        Some((_, player)) => &player.name,
        None => "Unknown Player",
    }
}

fn projectile_damage(amount: u16, ranged_strike: bool) -> Damage {
    if ranged_strike {
        Damage::strike(amount, true)
    } else {
        Damage::basic(amount)
    }
}

impl Effect {
    pub async fn affected_cards(&self) -> Option<Vec<CardId>> {
        match self {
            Effect::ShootProjectile { id, .. } => QueryCache::effect_targets(id),
            _ => None,
        }
    }

    pub async fn sound_effect(&self) -> anyhow::Result<Option<SoundEffect>> {
        let sound = match self {
            Effect::PlayCard { .. } => Some(SoundEffect::PlayCard),
            _ => None,
        };

        Ok(sound)
    }

    pub fn source_id(&self) -> Option<&uuid::Uuid> {
        match self {
            Effect::Noop => None,
            Effect::PlayerLost { player_id } => Some(player_id),
            Effect::SkipNextTurn { player_id } => Some(player_id),
            Effect::OverrideNextTurn { .. } => None,
            Effect::SetAvatarLife { player_id, .. } => Some(player_id),
            Effect::AdjustAvatarLife { player_id, .. } => Some(player_id),
            Effect::SummonToken { player_id, .. } => Some(player_id),
            Effect::Heal { card_id, .. } => Some(card_id),
            Effect::ShootProjectile { player_id, .. } => Some(player_id),
            Effect::RemoveAbility { card_id, .. } => Some(card_id),
            Effect::RemoveStatus { card_id, .. } => Some(card_id),
            Effect::AddAbilityCounter { card_id, .. } => Some(card_id),
            Effect::AddStatusCounter { card_id, .. } => Some(card_id),
            Effect::AddCounter { card_id, .. } => Some(card_id),
            Effect::SetCardRegion { card_id, .. } => Some(card_id),
            Effect::SetCardZone { card_id, .. } => Some(card_id),
            Effect::MoveCard { card_id, .. } => Some(card_id),
            Effect::DiscardCard { card_id, .. } => Some(card_id),
            Effect::DrawCard { player_id, .. } => Some(player_id),
            Effect::PlayMagic { card_id, .. } => Some(card_id),
            Effect::PlayCard { card_id, .. } => Some(card_id),
            Effect::SummonCards { .. } => None,
            Effect::SetTapped { card_id, .. } => Some(card_id),
            Effect::EndTurn { player_id } => Some(player_id),
            Effect::FinishEndTurn { player_id } => Some(player_id),
            Effect::StartTurn { player_id } => Some(player_id),
            Effect::AdjustMana { player_id, .. } => Some(player_id),
            Effect::Strike { striker_id, .. } => Some(striker_id),
            Effect::Attack { attacker_id, .. } => Some(attacker_id),
            Effect::RemoveCardFromGame { card_id } => Some(card_id),
            Effect::TakeDamage { card_id, .. } => Some(card_id),
            Effect::BanishCard { card_id, .. } => Some(card_id),
            Effect::KillMinion { card_id, .. } => Some(card_id),
            Effect::BuryCard { card_id, .. } => Some(card_id),
            Effect::Animate { card_id, .. } => Some(card_id),
            Effect::SetCardData { card_id, .. } => Some(card_id),
            Effect::TeleportCard { player_id, .. } => Some(player_id),
            Effect::RearrangeDeck { .. } => None,
            Effect::AddDeferredEffect { .. } => None,
            Effect::AddTemporaryEffect { .. } => None,
            Effect::SetBearer { card_id, .. } => Some(card_id),
            Effect::ShuffleDeck { .. } => None,
            Effect::SetController { card_id, .. } => Some(card_id),
            Effect::MakeCardCopyOf { card_id, .. } => Some(card_id),
            Effect::CopyMagic { card_id, .. } => Some(card_id),
            Effect::CopyArtifact { artifact_id, .. } => Some(artifact_id),
            Effect::SummonCopy { player_id, .. } => Some(player_id),
        }
    }

    /// Returns the card ID if this effect represents a card being played from hand
    /// (PlayCard or PlayMagic), so clients can display the card face to all players.
    pub fn played_card_id(&self) -> Option<CardId> {
        match self {
            Effect::PlayCard { card_id, .. } => Some(*card_id),
            Effect::PlayMagic { card_id, .. } => Some(*card_id),
            _ => None,
        }
    }

    pub async fn description(&self, state: &State) -> anyhow::Result<Option<String>> {
        let desc = match self {
            Effect::Noop => None,
            Effect::PlayerLost { player_id } => Some(format!(
                "{} has lost the game",
                player_name(player_id, state)
            )),
            Effect::SkipNextTurn { player_id } => Some(format!(
                "{} skips their next turn",
                player_name(player_id, state)
            )),
            Effect::OverrideNextTurn { turn } => match turn.controller_override() {
                Some(controller_id) => Some(format!(
                    "{} will control {}'s next turn",
                    player_name(&controller_id, state),
                    player_name(&turn.player_id(), state),
                )),
                None => Some(format!(
                    "{} will control their next turn",
                    player_name(&turn.player_id(), state),
                )),
            },
            Effect::SetAvatarLife { player_id, life } => Some(format!(
                "{}'s Avatar life becomes {}",
                player_name(player_id, state),
                life
            )),
            Effect::AdjustAvatarLife { player_id, amount } => {
                let player = player_name(player_id, state);
                if *amount >= 0 {
                    Some(format!("{} gains {} life", player, amount))
                } else {
                    Some(format!("{} loses {} life", player, amount.saturating_abs()))
                }
            }
            Effect::SetCardRegion {
                card_id,
                destination: region,
                ..
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!("{} changes region to {}", card, region))
            }
            Effect::AddTemporaryEffect { .. } => None,
            Effect::AddDeferredEffect { .. } => None,
            Effect::SummonToken {
                player_id,
                token_type,
                zone,
            } => {
                let token_name = match token_type {
                    TokenType::Rubble => "Rubble",
                    TokenType::FootSoldier => "Foot Soldier",
                    TokenType::Frog => "Frog",
                };
                Some(format!(
                    "{} summons a {} in zone {}",
                    player_name(player_id, state),
                    token_name,
                    zone
                ))
            }
            Effect::Heal { card_id, amount } => {
                let card = state.get_card(card_id).get_name();
                Some(format!("{} heals {} for {} health", card, card, amount))
            }
            Effect::RemoveAbility { .. } => None,
            Effect::RemoveStatus { .. } => None,
            Effect::AddStatusCounter { .. } => None,
            Effect::ShootProjectile {
                player_id,
                shooter,
                damage,
                direction,
                ..
            } => {
                let shooter_card = state.get_card(shooter);
                let shooter_name = shooter_card.get_name();
                let board_flipped = shooter_card.get_controller_id(state) != state.player_one;
                Some(format!(
                    "{} shoots a projectile for {} damage from {} in direction {}",
                    player_name(player_id, state),
                    damage,
                    shooter_name,
                    direction.normalise(board_flipped).get_name()
                ))
            }
            Effect::AddAbilityCounter { .. } => None,
            Effect::AddCounter { card_id, counter } => {
                let card = state.get_card(card_id).get_name();
                let fmt = |v: i16| {
                    if v >= 0 {
                        format!("+{}", v)
                    } else {
                        format!("{}", v)
                    }
                };
                Some(format!(
                    "{} gets a {}/{} counter",
                    card,
                    fmt(counter.power),
                    fmt(counter.toughness)
                ))
            }
            Effect::SetCardZone { .. } => None,
            Effect::DiscardCard { card_id, player_id } => {
                let card = state.get_card(card_id).get_name();
                Some(format!(
                    "{} discards {}",
                    player_name(player_id, state),
                    card
                ))
            }
            Effect::MoveCard {
                player_id,
                to,
                through_path,
                card_id,
                from,
                ..
            } => {
                let card = state.get_card(card_id);
                // If the card is still at `from`, the move was a no-op — don't log it
                if card.get_zone().location() == Some(from) {
                    return Ok(None);
                }

                let card_name = card.get_name();
                match through_path {
                    Some(path) => Some(format!(
                        "{} moves {} through {}",
                        player_name(player_id, state),
                        card_name,
                        path.iter()
                            .map(|c| format!("{}", c))
                            .collect::<Vec<_>>()
                            .join(" → "),
                    )),
                    None => Some(format!(
                        "{} moves {} to {}",
                        player_name(player_id, state),
                        card_name,
                        to.pick(player_id, state).await?.into_zone(),
                    )),
                }
            }
            Effect::DrawCard {
                player_id,
                count,
                kind,
            } => {
                if *count == 0 {
                    return Ok(None);
                }
                let cards = match kind {
                    DrawKind::Site if *count == 1 => "site",
                    DrawKind::Site => "sites",
                    DrawKind::Spell if *count == 1 => "spell",
                    DrawKind::Spell => "spells",
                    DrawKind::Choice if *count == 1 => "card",
                    DrawKind::Choice => "cards",
                };
                Some(format!(
                    "{} draws {} {}",
                    player_name(player_id, state),
                    count,
                    cards
                ))
            }
            Effect::PlayMagic {
                player_id, card_id, ..
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!("{} casts {}", player_name(player_id, state), card))
            }
            Effect::PlayCard {
                player_id,
                card_id,
                zone,
                ..
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!(
                    "{} plays {} in zone {}",
                    player_name(player_id, state),
                    card,
                    zone.pick(player_id, state).await?,
                ))
            }
            Effect::SummonCards { cards } => {
                if cards.is_empty() {
                    None
                } else {
                    let parts: Vec<String> = cards
                        .iter()
                        .map(|(player_id, card_id, _from_zone, location)| {
                            format!(
                                "{} summons {} in {}",
                                player_name(player_id, state),
                                state.get_card(card_id).get_name(),
                                location
                            )
                        })
                        .collect();
                    Some(parts.join("; "))
                }
            }
            Effect::SetTapped { .. } => None,
            Effect::EndTurn { player_id, .. } => {
                Some(format!("{} passes the turn", player_name(player_id, state)))
            }
            Effect::FinishEndTurn { .. } => None,
            Effect::StartTurn { player_id } => Some(format!(
                "--- {}'s turn begins ---",
                player_name(player_id, state)
            )),
            Effect::AdjustMana { .. } => None,
            Effect::Strike {
                striker_id,
                target_id,
            } => Some(format!(
                "{} strikes {} with {}",
                player_name(&state.get_card(striker_id).get_controller_id(state), state),
                state.get_card(target_id).get_name(),
                state.get_card(striker_id).get_name(),
            )),
            Effect::Attack {
                attacker_id,
                defender_id,
                ..
            } => {
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let player = player_name(&attacker.get_controller_id(state), state);
                Some(format!(
                    "{} attacks {} with {}",
                    player,
                    defender.get_name(),
                    attacker.get_name()
                ))
            }
            Effect::TakeDamage {
                card_id,
                from,
                damage,
                ..
            } => {
                let attacker = state.get_card(from).get_name();
                let defender = state.get_card(card_id).get_name();
                Some(format!(
                    "{} takes {} damage from {}",
                    defender, damage.amount, attacker
                ))
            }
            Effect::KillMinion {
                card_id, killer_id, ..
            } => {
                let card = state.get_card(card_id);
                let killer = state.get_card(killer_id);
                Some(format!("{} kills {}", killer.get_name(), card.get_name()))
            }
            Effect::BuryCard { card_id, .. } => {
                let card = state.get_card(card_id);
                let player = card.get_controller_id(state);
                Some(format!(
                    "{} buries {}",
                    player_name(&player, state),
                    card.get_name()
                ))
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.get_card(card_id);
                let player = card.get_controller_id(state);
                Some(format!(
                    "{} banishes {}",
                    player_name(&player, state),
                    card.get_name()
                ))
            }
            Effect::Animate { card_id, .. } => {
                let card = state.get_card(card_id);
                Some(format!("{} becomes a minion", card.get_name()))
            }
            Effect::SetCardData { .. } => None,
            Effect::TeleportCard {
                player_id,
                card_id,
                to_location,
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!(
                    "{} teleports {} to {}",
                    player_name(player_id, state),
                    card,
                    to_location
                ))
            }
            Effect::RearrangeDeck { .. } => None,
            Effect::SetBearer { card_id, bearer_id } => {
                let card = state.get_card(card_id);
                match bearer_id {
                    Some(carrier_id) => {
                        let carrier = state.get_card(carrier_id);
                        Some(format!(
                            "{} is now carried by {}",
                            card.get_name(),
                            carrier.get_name()
                        ))
                    }
                    None => Some(format!("{} is no longer carried", card.get_name())),
                }
            }
            Effect::ShuffleDeck { player_id } => Some(format!(
                "{} shuffles their deck",
                player_name(player_id, state)
            )),
            Effect::SetController { card_id, player_id } => {
                let card_name = state.get_card(card_id).get_name();
                Some(format!(
                    "{} gains control of {}",
                    player_name(player_id, state),
                    card_name
                ))
            }
            Effect::MakeCardCopyOf {
                card_id,
                copy_source_id,
            } => Some(format!(
                "{} becomes a copy of {}",
                state.get_card(card_id).get_name(),
                state.get_card(copy_source_id).get_name()
            )),
            Effect::CopyMagic {
                player_id, card_id, ..
            } => Some(format!(
                "{} copies {}",
                player_name(player_id, state),
                state.get_card(card_id).get_name()
            )),
            Effect::CopyArtifact {
                player_id,
                artifact_id,
                ..
            } => Some(format!(
                "{} creates a copy of {}",
                player_name(player_id, state),
                state.get_card(artifact_id).get_name(),
            )),
            Effect::SummonCopy {
                card_name,
                player_id,
                zone,
            } => Some(format!(
                "{} summons a copy of {} in {}",
                player_name(player_id, state),
                card_name,
                zone
            )),
            Effect::RemoveCardFromGame { .. } => None,
        };

        Ok(desc)
    }

    async fn expire_counters(&self, state: &mut State) -> anyhow::Result<()> {
        let modified_cards: Vec<&dyn Card> = state
            .cards
            .values()
            .filter(|c| c.is_unit())
            .filter(|c| {
                !c.get_unit_base()
                    .expect("unit to have a unit base component")
                    .ability_counters
                    .is_empty()
            })
            .map(|c| c.as_ref())
            .collect();
        let mut card_modifiers_to_remove: Vec<(uuid::Uuid, Vec<CardId>)> = vec![];
        for card in modified_cards {
            let mut to_remove: Vec<CardId> = vec![];
            for counter in &card
                .get_unit_base()
                .unwrap_or(&UnitBase::default())
                .ability_counters
            {
                if let Some(effect_query) = &counter.expires_on_effect
                    && effect_query.matches(self, state).await?
                {
                    to_remove.push(counter.id);
                }
            }

            if !to_remove.is_empty() {
                card_modifiers_to_remove.push((*card.get_id(), to_remove));
            }
        }

        for (card_id, to_remove) in card_modifiers_to_remove {
            let card_mut = state.get_card_mut(&card_id);
            for counter_id in to_remove {
                card_mut.remove_modifier_counter(&counter_id);
            }
        }

        let modified_cards: Vec<&dyn Card> = state
            .cards
            .values()
            .filter(|c| !c.get_base().status_counters.is_empty())
            .map(|c| c.as_ref())
            .collect();
        let mut card_statuses_to_remove: Vec<(uuid::Uuid, Vec<CardId>)> = vec![];
        for card in modified_cards {
            let mut to_remove: Vec<CardId> = vec![];
            for counter in &card.get_base().status_counters {
                if let Some(effect_query) = &counter.expires_on_effect
                    && effect_query.matches(self, state).await?
                {
                    to_remove.push(counter.id);
                }
            }

            if !to_remove.is_empty() {
                card_statuses_to_remove.push((*card.get_id(), to_remove));
            }
        }

        for (card_id, to_remove) in card_statuses_to_remove {
            let card_mut = state.get_card_mut(&card_id);
            for counter_id in to_remove {
                card_mut.remove_status_counter(&counter_id);
            }
        }

        let cards_with_counters: Vec<&dyn Card> = state
            .cards
            .values()
            .filter(|c| c.is_unit())
            .filter(|c| {
                !c.get_unit_base()
                    .unwrap_or(&UnitBase::default())
                    .power_counters
                    .is_empty()
            })
            .map(|c| c.as_ref())
            .collect();
        let mut card_counters_to_remove: Vec<(uuid::Uuid, Vec<CardId>)> = vec![];
        for card in cards_with_counters {
            let mut to_remove: Vec<CardId> = vec![];
            for counter in &card
                .get_unit_base()
                .unwrap_or(&UnitBase::default())
                .power_counters
            {
                if let Some(effect_query) = &counter.expires_on_effect
                    && effect_query.matches(self, state).await?
                {
                    to_remove.push(counter.id);
                }
            }

            if !to_remove.is_empty() {
                card_counters_to_remove.push((*card.get_id(), to_remove));
            }
        }

        for (card_id, to_remove) in card_counters_to_remove {
            let card_mut = state.get_card_mut(&card_id);
            for counter_id in to_remove {
                card_mut.remove_power_counter(&counter_id);
            }
        }

        Ok(())
    }

    pub async fn apply(&self, state: &mut State) -> anyhow::Result<()> {
        state.invalidate_runtime_caches();

        let mut effect = self.clone();
        EffectLifecycle::modify_effect(state, &mut effect).await?;

        match &effect {
            Effect::Noop => {}
            Effect::PlayerLost { player_id } => {
                state.eliminate_player(*player_id);
            }
            Effect::SkipNextTurn { player_id } => {
                state.skip_next_turn_for(player_id);
            }
            Effect::OverrideNextTurn { turn } => {
                state.override_next_turn(turn.clone());
            }
            Effect::SetAvatarLife { player_id, life } => {
                let avatar_id = state.get_player_avatar_id(player_id)?;
                let avatar = state.get_card_mut(&avatar_id);
                if avatar
                    .get_avatar_base()
                    .is_some_and(|avatar_base| avatar_base.deaths_door)
                {
                    return Ok(());
                }
                let unit_base = avatar
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("avatar has no unit base component"))?;
                unit_base.damage = unit_base.toughness.saturating_sub(*life);
                if unit_base.damage >= unit_base.toughness {
                    let avatar_base = avatar
                        .get_avatar_base_mut()
                        .ok_or(anyhow::anyhow!("avatar has no avatar base component"))?;
                    avatar_base.deaths_door = true;
                }
            }
            Effect::AdjustAvatarLife { player_id, amount } => {
                let avatar_id = state.get_player_avatar_id(player_id)?;
                let avatar = state.get_card_mut(&avatar_id);
                if avatar
                    .get_avatar_base()
                    .is_some_and(|avatar_base| avatar_base.deaths_door)
                {
                    return Ok(());
                }

                let unit_base = avatar
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("avatar has no unit base component"))?;
                let current_life = unit_base.toughness.saturating_sub(unit_base.damage);
                let new_life = if *amount >= 0 {
                    current_life.saturating_add(*amount as u16)
                } else {
                    current_life.saturating_sub(amount.saturating_abs() as u16)
                }
                .min(unit_base.toughness);

                unit_base.damage = unit_base.toughness.saturating_sub(new_life);
                if unit_base.damage >= unit_base.toughness {
                    let avatar_base = avatar
                        .get_avatar_base_mut()
                        .ok_or(anyhow::anyhow!("avatar has no avatar base component"))?;
                    avatar_base.deaths_door = true;
                }
            }
            Effect::AddDeferredEffect { effect, .. } => {
                state.deferred_effects_mut().push(effect.clone());
            }
            Effect::AddTemporaryEffect { effect } => {
                state.temporary_effects_mut().push(effect.clone());
                state.queue(location_survival_effects_for_realm(state));
            }
            Effect::SetCardZone { card_id, zone } => {
                let was_in_play = state.get_card(card_id).get_zone().is_in_play();
                let original_zone = state.get_card(card_id).get_zone().clone();
                let owner_id = *state.get_card(card_id).get_owner_id();
                let is_token = state.get_card(card_id).is_token();
                let mut ongoing_effects_changed = false;
                let card = state.get_card_mut(card_id);
                card.set_zone(zone.clone());
                if was_in_play && !zone.is_in_play() {
                    state.remove_ongoing_effects_from_source(card_id);
                    ongoing_effects_changed = true;
                } else if !was_in_play && zone.is_in_play() {
                    state
                        .add_passive_ongoing_effects_for_source(card_id)
                        .await?;
                    ongoing_effects_changed = true;
                    if let Some(mana_effect) =
                        mana_effect_for_resource_entering_realm(state, card_id)?
                    {
                        state.queue_one(mana_effect);
                    }
                }
                match original_zone {
                    Zone::Spellbook => {
                        state
                            .get_player_deck_mut(&owner_id)?
                            .spells
                            .retain(|id| id != card_id);
                    }
                    Zone::Atlasbook => {
                        state
                            .get_player_deck_mut(&owner_id)?
                            .sites
                            .retain(|id| id != card_id);
                    }
                    _ => {}
                }
                // Tokens cease to exist when they leave the realm.
                if was_in_play && !zone.is_in_play() && is_token {
                    state.queue_one(Effect::RemoveCardFromGame { card_id: *card_id });
                }
                if ongoing_effects_changed {
                    state.queue(location_survival_effects_for_realm(state));
                } else {
                    state.queue(location_survival_effects_for_zones(
                        state,
                        [original_zone, zone.clone()],
                    ));
                }
            }
            Effect::SummonToken {
                player_id,
                token_type,
                zone,
            } => {
                let token: Box<dyn Card> = match token_type {
                    TokenType::Rubble => Box::new(Rubble::new(*player_id)),
                    TokenType::FootSoldier => Box::new(FootSoldier::new(*player_id)),
                    TokenType::Frog => Box::new(Frog::new(*player_id)),
                };

                if token.is_unit() {
                    // Unit tokens are summoned via SummonCards so that zone placement,
                    // SummoningSickness, summon hooks, and genesis all happen in one place.
                    let token: Box<dyn Card> = match token_type {
                        TokenType::FootSoldier => Box::new(FootSoldier::new(*player_id)),
                        TokenType::Frog => Box::new(Frog::new(*player_id)),
                        TokenType::Rubble => unreachable!(),
                    };
                    let token_id = *token.get_id();
                    state.cards.insert(token_id, token);
                    state.invalidate_runtime_caches();
                    state.queue_one(Effect::SummonCards {
                        cards: vec![(
                            *player_id,
                            token_id,
                            Zone::None,
                            zone.clone()
                                .into_location()
                                .ok_or(anyhow::anyhow!("token summon zone must be a location"))?,
                        )],
                    });
                } else {
                    // Non-unit tokens are just placed directly onto the board without going through
                    // SummonCards, since they don't need to trigger any summon hooks or genesis effects.
                    let mut token = token;
                    let token_id = *token.get_id();
                    token.set_zone(zone.clone());
                    state.cards.insert(token_id, token);
                    state
                        .add_passive_ongoing_effects_for_source(&token_id)
                        .await?;
                    state.invalidate_runtime_caches();
                    if zone.is_in_play()
                        && let Some(mana_effect) =
                            mana_effect_for_resource_entering_realm(state, &token_id)?
                    {
                        state.queue_one(mana_effect);
                    }
                }
            }
            Effect::Heal { card_id, amount } => {
                let card = state.get_card_mut(card_id);
                if card
                    .get_avatar_base()
                    .is_some_and(|avatar_base| avatar_base.deaths_door)
                {
                    return Ok(());
                }
                let unit_base = card
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("card has no unit base"))?;
                unit_base.damage = unit_base.damage.saturating_sub(*amount);
            }
            Effect::RemoveAbility { card_id, modifier } => {
                let card = state.get_card_mut(card_id);
                card.remove_modifier(modifier);
            }
            Effect::RemoveStatus { card_id, status } => {
                let card = state.get_card_mut(card_id);
                card.remove_status(status);
            }
            Effect::ShootProjectile {
                id,
                range,
                player_id,
                shooter,
                from_zone,
                direction,
                damage,
                ranged_strike,
                piercing,
                splash_damage,
                ..
            } => {
                state
                    .get_sender()
                    .send(ServerMessage::ProjectileFired {
                        player_id: *player_id,
                        shooter: *shooter,
                        from_zone: from_zone.clone(),
                        direction: direction.clone(),
                        range: *range,
                        ranged_strike: *ranged_strike,
                    })
                    .await?;

                let mut effects = vec![];
                let mut next_zone = Some(from_zone.clone());
                let mut is_starting_location = true;
                let mut range: Option<u8> = *range;
                while let Some(zone) = next_zone {
                    // Check if the projectile is out of range. If not, decrease the remaning range.
                    if !is_starting_location && let Some(steps) = range.as_mut() {
                        if *steps == 0 {
                            break;
                        }

                        *steps -= 1;
                    }

                    let picked_unit_id = match self.affected_cards().await {
                        Some(affected_cards) => affected_cards.first().cloned(),
                        None => {
                            let mut units_query = CardQuery::new()
                                .units()
                                .in_zone(&zone)
                                .can_be_targeted_by_player(player_id);
                            // Allied units in the starting location are ignored by projectiles.
                            if is_starting_location {
                                units_query =
                                    units_query.controlled_by(&state.get_opponent_id(player_id)?);
                            }

                            let units = units_query.all(state);
                            match units.len() {
                                0 => None,
                                1 => Some(units[0]),
                                _ => {
                                    let prompt = "Pick a unit to shoot";
                                    let picked_unit_id =
                                        pick_card(player_id, &units, state, prompt).await?;
                                    QueryCache::store_effect_targets(
                                        state.game_id,
                                        *id,
                                        vec![picked_unit_id],
                                    );
                                    Some(picked_unit_id)
                                }
                            }
                        }
                    };

                    if let Some(picked_unit_id) = picked_unit_id {
                        effects.push(Effect::TakeDamage {
                            card_id: picked_unit_id,
                            from: *shooter,
                            damage: projectile_damage(*damage, *ranged_strike),
                        });
                        if let Some(splash_damage) = splash_damage {
                            let splash_effects = CardQuery::new()
                                .units()
                                .in_zone(&zone)
                                .id_not(&picked_unit_id)
                                .all(state)
                                .into_iter()
                                .map(|c| Effect::TakeDamage {
                                    card_id: c,
                                    from: *shooter,
                                    damage: Damage::basic(*splash_damage),
                                })
                                .collect::<Vec<_>>();
                            effects.extend(splash_effects);
                        }

                        if !piercing {
                            break;
                        }
                    }

                    next_zone = zone.zone_in_direction(direction, 1);
                    is_starting_location = false;
                }

                for effect in effects {
                    state.queue_one(effect);
                }
            }
            Effect::MoveCard {
                player_id,
                card_id,
                from,
                to,
                tap,
                through_path,
            } => {
                let card = state.get_card(card_id);
                // Skip the move if the card is no longer in the same zone as it was originally.
                if card.get_zone().location() != Some(from) {
                    return Ok(());
                }

                // If this card was being carried, it no longer is.
                state.get_card_mut(card_id).set_bearer_id(None);

                match through_path {
                    Some(path) => {
                        for path_zone in path.iter() {
                            let zone = ZoneQuery::from_zone(path_zone.clone())
                                .pick(player_id, state)
                                .await?;
                            let card = state.get_card_mut(card_id);
                            card.set_zone(zone.clone());
                            if *tap {
                                card.set_tapped(true);
                            }

                            // Move carried minions along with the carrier
                            let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                            for cid in &carried_cards {
                                let carried_card = state.get_card_mut(cid);
                                carried_card.set_zone(zone.clone());
                            }

                            let mut effects = vec![];
                            effects.extend(location_survival_effects_for_cards(
                                state,
                                std::iter::once(*card_id).chain(carried_cards.iter().copied()),
                            ));

                            state.queue(effects);
                        }
                    }
                    None => {
                        let zone = to.pick(player_id, state).await?.into_zone();
                        let card = state.get_card_mut(card_id);
                        card.set_zone(zone.clone());
                        if *tap {
                            card.set_tapped(true);
                        }

                        // Move carried minions along with the carrier
                        let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                        for cid in &carried_cards {
                            let carried_card = state.get_card_mut(cid);
                            carried_card.set_zone(zone.clone());
                        }

                        let mut effects = vec![];
                        effects.extend(location_survival_effects_for_cards(
                            state,
                            std::iter::once(*card_id).chain(carried_cards.iter().copied()),
                        ));

                        state.queue(effects);
                    }
                }
            }
            Effect::DrawCard {
                player_id,
                count,
                kind,
            } => {
                for _ in 0..*count {
                    let kind = match kind {
                        DrawKind::Choice => {
                            let options: Vec<DrawKind> = vec![DrawKind::Site, DrawKind::Spell];
                            let option_labels = options
                                .iter()
                                .map(|kind| match kind {
                                    DrawKind::Site => "Draw Site".to_string(),
                                    DrawKind::Spell => "Draw Spell".to_string(),
                                    DrawKind::Choice => unreachable!(),
                                })
                                .collect::<Vec<_>>();
                            let picked_option_idx =
                                pick_option(player_id, &option_labels, state, "Draw a card", false)
                                    .await?;
                            options[picked_option_idx].clone()
                        }
                        kind => kind.clone(),
                    };
                    let card_id = {
                        let deck = state
                            .decks
                            .get_mut(player_id)
                            .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                        match kind {
                            DrawKind::Site => deck.sites.pop(),
                            DrawKind::Spell => deck.spells.pop(),
                            DrawKind::Choice => unreachable!(),
                        }
                    };

                    if let Some(card_id) = card_id {
                        state
                            .cards
                            .values_mut()
                            .find(|c| c.get_id() == &card_id)
                            .expect("to find drawn card")
                            .set_zone(Zone::Hand);
                    } else {
                        state.queue_one(Effect::PlayerLost {
                            player_id: *player_id,
                        });
                        break;
                    }
                }
            }
            Effect::PlayMagic {
                card_id,
                player_id,
                caster_id,
                ..
            } => {
                // Casting a spell is an interaction: the caster loses Stealth.
                state
                    .get_card_mut(caster_id)
                    .remove_modifier(&Ability::Stealth);

                let costs = state.get_effective_costs(card_id, None, player_id)?;
                let paid_cost = costs.pay(state, player_id).await?;

                let snapshot = state.clone();
                let card = state.get_card_mut(card_id);
                card.set_controller_id(player_id);
                let effects = card
                    .get_magic()
                    .ok_or(anyhow::anyhow!("magic card does not implement magic"))?
                    .resolve_magic(&snapshot, caster_id, paid_cost)
                    .await?;

                // Set zone after resolving so that the card is not in the cemetery during casting.
                card.set_zone(Zone::Cemetery);
                state.queue(effects);
            }
            Effect::PlayCard {
                card_id,
                player_id,
                zone,
                ..
            } => {
                let zone = zone.pick(player_id, state).await?;
                let costs = state.get_effective_costs(card_id, Some(&zone), player_id)?;
                Box::pin(costs.pay(state, player_id)).await?;
                let card = state.get_card(card_id);
                let is_minion = card.is_minion();
                let snapshot = state.clone();

                // If playing a site and there is a rubble on that zone, remove it.
                {
                    let card = state
                        .cards
                        .values()
                        .find(|c| c.get_id() == card_id)
                        .expect("to find card");
                    if card.is_site()
                        && let Some(site) = zone.get_site(&snapshot)
                        && site.get_name() == Rubble::NAME
                    {
                        state.queue_one(Effect::RemoveCardFromGame {
                            card_id: *site.get_id(),
                        });
                    }
                }

                if is_minion {
                    // Minions are put into play via SummonCards, which handles zone
                    // placement, SummoningSickness, summon hooks, and genesis in one place.
                    state.queue_one(Effect::SummonCards {
                        cards: vec![(
                            *player_id,
                            *card_id,
                            Zone::Hand,
                            zone.into_location()
                                .ok_or(anyhow::anyhow!("play zone must be a location"))?,
                        )],
                    });
                } else {
                    let from_zone = {
                        let card = state
                            .cards
                            .values_mut()
                            .find(|c| c.get_id() == card_id)
                            .expect("to find card");
                        card.set_controller_id(player_id);
                        let from_zone = card.get_zone().clone();
                        card.set_zone(zone.clone());
                        from_zone
                    };

                    // Sync state for all palyers so that genesis effects see the card in the board
                    // when it triggers.
                    crate::game::force_sync_all(state).await?;

                    state
                        .add_passive_ongoing_effects_for_source(card_id)
                        .await?;
                    let card = state.get_card(card_id);
                    let mut effects = card.genesis(state).await?;
                    if !from_zone.is_in_play()
                        && zone.is_in_play()
                        && let Some(mana_effect) =
                            mana_effect_for_resource_entering_realm(state, card_id)?
                    {
                        effects.push(mana_effect);
                    }
                    effects.extend(location_survival_effects_for_realm(state));
                    state.queue(effects);
                }
            }
            Effect::SummonCards { cards } => {
                let snapshot = state.clone();
                for (player_id, card_id, _from_zone, location) in cards {
                    let zone = location.clone().into_zone();
                    let has_charge = state.get_card(card_id).has_ability(state, &Ability::Charge);
                    let original_zone = state.get_card(card_id).get_zone().clone();
                    let owner_id = *state.get_card(card_id).get_owner_id();
                    {
                        let card = state.get_card_mut(card_id);
                        card.set_controller_id(player_id);
                        card.set_zone(zone.clone());
                        if !has_charge {
                            card.add_status(CardStatus::SummoningSickness);
                        }
                    }

                    // Sync state for all palyers so that genesis effects see the card in the board
                    // when it triggers.
                    crate::game::force_sync_all(state).await?;

                    if !original_zone.is_in_play() && zone.is_in_play() {
                        state
                            .add_passive_ongoing_effects_for_source(card_id)
                            .await?;
                    } else if original_zone.is_in_play() && !zone.is_in_play() {
                        state.remove_ongoing_effects_from_source(card_id);
                    }
                    match original_zone {
                        Zone::Spellbook => {
                            state
                                .get_player_deck_mut(&owner_id)?
                                .spells
                                .retain(|id| id != card_id);
                        }
                        Zone::Atlasbook => {
                            state
                                .get_player_deck_mut(&owner_id)?
                                .sites
                                .retain(|id| id != card_id);
                        }
                        _ => {}
                    }
                }
                state.invalidate_runtime_caches();

                // Force sync after all cards have been put on their zones, so that players see them
                // on the board while resolving effects from summon hooks, genesis and on_visit_zone.
                crate::game::force_sync_all(state).await?;

                let mut effects = vec![];
                for (_, card_id, _from_zone, location) in cards {
                    let zone = location.clone().into_zone();
                    let card = state.get_card(card_id);
                    let from_zone = snapshot.get_card(card_id).get_zone().clone();
                    effects.extend(card.genesis(state).await?);
                    if !from_zone.is_in_play()
                        && zone.is_in_play()
                        && let Some(mana_effect) =
                            mana_effect_for_resource_entering_realm(state, card_id)?
                    {
                        effects.push(mana_effect);
                    }
                    effects.extend(location_survival_effects_for_cards(
                        state,
                        std::iter::once(*card_id),
                    ));
                }

                state.queue(effects);
            }
            Effect::SetTapped {
                card_id, tapped, ..
            } => {
                let card = state
                    .cards
                    .values_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                card.set_tapped(*tapped);
            }
            Effect::StartTurn { player_id, .. } => {
                let previous_controller = state.current_turn_controller();
                let turn = state.advance_to_turn(player_id)?;
                let turn_controller = turn.controller_id();
                state
                    .get_sender()
                    .send(ServerMessage::Wait {
                        player_id: previous_controller,
                        prompt: "Waiting for other player".to_string(),
                    })
                    .await?;

                // Snapshot for controller checks (get_controller_id needs &State).
                let ctrl_snapshot = state.clone();
                // Untap cards controlled by the current player (not merely owned — control can
                // be transferred via steal effects).
                let controlled_cards: Vec<CardId> = state
                    .cards
                    .values()
                    .filter(|c| &c.get_controller_id(&ctrl_snapshot) == player_id)
                    .map(|c| *c.get_id())
                    .collect();
                for cid in controlled_cards {
                    state.get_card_mut(&cid).set_tapped(false);
                }

                // Mana is generated by sites the current player controls (not merely owns).
                let available_mana: u8 = state
                    .cards
                    .values()
                    .filter(|c| &c.get_controller_id(&ctrl_snapshot) == player_id)
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| {
                        c.get_resource_provider().map(|rp| {
                            rp.provided_mana(&ctrl_snapshot)
                                .expect("to get provided mana")
                        })
                    })
                    .sum();
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = available_mana;

                let mut all_effects: Vec<Effect> = vec![];
                for card in state.cards.values().filter(|c| {
                    c.get_zone().is_in_play() && can_use_special_abilities(state, c.get_id())
                }) {
                    let effects = card.on_turn_start(state).await?;
                    all_effects.extend(effects);
                }
                state.queue(all_effects);

                // The first player skips their draw on the very first turn of the game.
                let is_first_players_first_turn =
                    state.turns == 0 && player_id == &state.player_one;
                if !is_first_players_first_turn {
                    let options: Vec<BaseAction> =
                        vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                    let option_labels: Vec<String> =
                        options.iter().map(|a| a.get_name().to_string()).collect();
                    let prompt = "Start Turn: Pick card to draw";
                    let picked_option_idx =
                        pick_option(turn_controller, &option_labels, state, prompt, false).await?;
                    let effects = options[picked_option_idx]
                        .on_select(player_id, state)
                        .await?;
                    state.queue(effects);
                }

                state.turns += 1;
                state
                    .get_sender()
                    .send(ServerMessage::Resume {
                        player_id: previous_controller,
                    })
                    .await?;
            }
            Effect::AdjustMana {
                player_id, mana, ..
            } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = ((*player_mana as i8) + *mana) as u8;
            }
            Effect::EndTurn { player_id, .. } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = 0;
                state.phase = Phase::Main;

                state.queue_front(Effect::FinishEndTurn {
                    player_id: *player_id,
                });
            }
            Effect::FinishEndTurn { .. } => {
                let cards = state.cards.values_mut().filter(|c| c.is_unit());
                for card in cards {
                    card.remove_status(&CardStatus::SummoningSickness);

                    if card.is_avatar() {
                        // Avatars at death's door become killable after the turn ends.
                        if let Some(ab) = card.get_avatar_base_mut()
                            && ab.deaths_door
                        {
                            ab.can_die = true;
                        }
                        continue;
                    }

                    card.get_unit_base_mut()
                        .ok_or(anyhow::anyhow!("card has no unit base component"))?
                        .damage = 0;
                }
                for effect in state.temporary_effects_mut() {
                    if let TemporaryEffect::Animate { unit_base, .. } = effect {
                        unit_base.damage = 0;
                    }
                }
                state.invalidate_runtime_caches();

                // Push StartTurn to the front of the queue so all end of turn effects are resolved
                // first.
                state.queue_front(Effect::StartTurn {
                    player_id: state.next_turn().player_id(),
                });
            }
            Effect::Strike {
                striker_id,
                target_id,
            } => {
                // Striking is an interaction: the striker loses Stealth.
                state
                    .get_card_mut(striker_id)
                    .remove_modifier(&Ability::Stealth);

                let snapshot = state.clone();
                let attacker = state.get_card(striker_id);
                if attacker.has_status(&snapshot, &CardStatus::Disabled) {
                    return Ok(());
                }

                state.queue_one(Effect::TakeDamage {
                    card_id: *target_id,
                    from: *striker_id,
                    damage: Damage::strike(
                        attacker
                            .get_power(&snapshot)?
                            .ok_or(anyhow::anyhow!("attacker has no power"))?,
                        false,
                    ),
                });
            }
            Effect::Attack {
                attacker_id,
                defender_id,
                defending_ids,
                damage_assignment,
                ..
            } => {
                // Attacking is an interaction: the attacker loses Stealth.
                state
                    .get_card_mut(attacker_id)
                    .remove_modifier(&Ability::Stealth);

                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);

                let mut effects = vec![];
                if !defending_ids.is_empty() || damage_assignment.is_some() {
                    let attacker_power = attacker
                        .get_power(state)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?;
                    let attacker_zone = attacker.get_zone().clone();
                    let attacker_has_fs = attacker.has_ability(state, &Ability::FirstStrike);

                    let mut assigned_damage = HashMap::new();
                    if let Some(damage_assignment) = damage_assignment {
                        let mut remaining_damage = attacker_power;
                        for defending_id in defending_ids {
                            let requested = damage_assignment
                                .get(defending_id)
                                .copied()
                                .unwrap_or_default();
                            let assigned = requested.min(remaining_damage);
                            remaining_damage -= assigned;
                            assigned_damage.insert(*defending_id, assigned);
                        }
                    } else {
                        for defending_id in defending_ids {
                            assigned_damage.insert(*defending_id, attacker_power);
                        }
                    }

                    effects.extend(
                        defending_ids
                            .iter()
                            .map(|defending_id| {
                                let defending_card = state.get_card(defending_id);
                                if defending_card.occupies_zone(state, &attacker_zone) {
                                    Effect::SetTapped {
                                        card_id: *defending_id,
                                        tapped: true,
                                    }
                                } else {
                                    Effect::MoveCard {
                                        player_id: defending_card.get_controller_id(state),
                                        card_id: *defending_id,
                                        from: defending_card
                                            .get_zone()
                                            .clone()
                                            .into_location()
                                            .expect("MoveCard source must be a location"),
                                        to: LocationQuery::from_zone(attacker_zone.clone()),
                                        tap: true,
                                        through_path: None,
                                    }
                                }
                            })
                            .collect::<Vec<_>>(),
                    );

                    let defenders_with_fs = defending_ids
                        .iter()
                        .copied()
                        .filter(|id| state.get_card(id).has_ability(state, &Ability::FirstStrike))
                        .collect::<Vec<_>>();
                    let has_first_strike_phase = attacker_has_fs || !defenders_with_fs.is_empty();

                    if has_first_strike_phase {
                        let mut first_strike_effects = Vec::new();
                        if attacker_has_fs {
                            for defending_id in defending_ids {
                                let damage =
                                    assigned_damage.get(defending_id).copied().unwrap_or(0);
                                first_strike_effects.push(Effect::TakeDamage {
                                    card_id: *defending_id,
                                    from: *attacker_id,
                                    damage: Damage::strike(damage, false),
                                });
                            }
                        }

                        for defending_id in &defenders_with_fs {
                            let defender = state.get_card(defending_id);
                            if defender.strikes_back(state)? {
                                let defender_power = defender
                                    .get_power(state)?
                                    .ok_or(anyhow::anyhow!("defender has no power"))?;
                                first_strike_effects.push(Effect::TakeDamage {
                                    card_id: *attacker_id,
                                    from: *defending_id,
                                    damage: Damage::strike(defender_power, false),
                                });
                            }
                        }

                        let mut sim = state.clone();
                        sim.queue(first_strike_effects.clone());
                        Box::pin(sim.apply_effects_without_log()).await?;
                        effects.extend(first_strike_effects);

                        let attacker_survived = sim
                            .cards
                            .get(attacker_id)
                            .is_some_and(|card| card.get_zone() != &Zone::Cemetery);
                        if attacker_survived && !attacker_has_fs {
                            for defending_id in defending_ids {
                                let defender_survived = sim
                                    .cards
                                    .get(defending_id)
                                    .is_some_and(|card| card.get_zone() != &Zone::Cemetery);
                                if !defender_survived {
                                    continue;
                                }
                                let damage =
                                    assigned_damage.get(defending_id).copied().unwrap_or(0);
                                effects.push(Effect::TakeDamage {
                                    card_id: *defending_id,
                                    from: *attacker_id,
                                    damage: Damage::strike(damage, false),
                                });
                            }
                        }

                        for defending_id in defending_ids {
                            if defenders_with_fs.contains(defending_id) {
                                continue;
                            }
                            let defender_survived = sim
                                .cards
                                .get(defending_id)
                                .is_some_and(|card| card.get_zone() != &Zone::Cemetery);
                            if !defender_survived {
                                continue;
                            }
                            let defender = state.get_card(defending_id);
                            if defender.strikes_back(state)? {
                                let defender_power = defender
                                    .get_power(state)?
                                    .ok_or(anyhow::anyhow!("defender has no power"))?;
                                effects.push(Effect::TakeDamage {
                                    card_id: *attacker_id,
                                    from: *defending_id,
                                    damage: Damage::strike(defender_power, false),
                                });
                            }
                        }
                    } else {
                        for defending_id in defending_ids {
                            let damage = assigned_damage.get(defending_id).copied().unwrap_or(0);
                            effects.push(Effect::TakeDamage {
                                card_id: *defending_id,
                                from: *attacker_id,
                                damage: Damage::strike(damage, false),
                            });
                        }

                        for defending_id in defending_ids {
                            let defender = state.get_card(defending_id);
                            if defender.strikes_back(state)? {
                                let defender_power = defender
                                    .get_power(state)?
                                    .ok_or(anyhow::anyhow!("defender has no power"))?;
                                effects.push(Effect::TakeDamage {
                                    card_id: *attacker_id,
                                    from: *defending_id,
                                    damage: Damage::strike(defender_power, false),
                                });
                            }
                        }
                    }
                    effects.reverse();
                    state.queue(effects);
                    return Ok(());
                }

                if !attacker.occupies_zone(state, defender.get_zone()) {
                    effects.push(Effect::MoveCard {
                        player_id: attacker.get_controller_id(state),
                        card_id: *attacker_id,
                        from: attacker
                            .get_zone()
                            .clone()
                            .into_location()
                            .expect("MoveCard source must be a location"),
                        to: defender.get_zone().into(),
                        tap: true,
                        through_path: None,
                    });
                }

                let attacker_has_fs = attacker.has_ability(state, &Ability::FirstStrike);
                let defender_has_fs = defender.has_ability(state, &Ability::FirstStrike);
                if attacker_has_fs != defender_has_fs {
                    let first_attacker = if attacker_has_fs { attacker } else { defender };
                    let first_defender = if attacker_has_fs { defender } else { attacker };
                    let first_defender_survived = {
                        let mut sim = state.clone();
                        let power = first_attacker
                            .get_power(&sim)?
                            .ok_or(anyhow::anyhow!("first attacker has no power"))?;
                        sim.queue_one(Effect::TakeDamage {
                            card_id: *first_defender.get_id(),
                            from: *first_attacker.get_id(),
                            damage: Damage::strike(power, false),
                        });
                        Box::pin(sim.apply_effects_without_log()).await?;
                        sim.get_card(first_defender.get_id()).get_zone() != &Zone::Cemetery
                    };

                    let power = first_attacker
                        .get_power(state)?
                        .ok_or(anyhow::anyhow!("first defender has no power"))?;
                    effects.push(Effect::TakeDamage {
                        card_id: *first_defender.get_id(),
                        from: *first_attacker.get_id(),
                        damage: Damage::strike(power, false),
                    });

                    if first_defender_survived && first_defender.strikes_back(state)? {
                        let power = first_defender
                            .get_power(state)?
                            .ok_or(anyhow::anyhow!("first attacker has no power"))?;
                        effects.push(Effect::TakeDamage {
                            card_id: *first_attacker.get_id(),
                            from: *first_defender.get_id(),
                            damage: Damage::strike(power, false),
                        });
                    }
                } else {
                    // Both have FirstStrike or neither does: both strike simultaneously.
                    let attacker_power = attacker
                        .get_power(state)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?;
                    effects.push(Effect::TakeDamage {
                        card_id: *defender_id,
                        from: *attacker_id,
                        damage: Damage::strike(attacker_power, false),
                    });

                    if defender.strikes_back(state)? {
                        let defender_power = defender
                            .get_power(state)?
                            .ok_or(anyhow::anyhow!("defender has no power"))?;
                        effects.push(Effect::TakeDamage {
                            card_id: *attacker_id,
                            from: *defender_id,
                            damage: Damage::strike(defender_power, false),
                        });
                    }
                }

                effects.reverse();
                state.queue(effects);
            }
            Effect::TakeDamage {
                card_id,
                damage,
                from,
            } => {
                // The card dealing damage loses Stealth (it has revealed itself by interacting).
                state.get_card_mut(from).remove_modifier(&Ability::Stealth);

                let snapshot = state.clone();
                // Check if this card has DoubleDamageTaken applied to it.
                let takes_double_damage = snapshot.active_continuous_effects().into_iter().any(|ce| {
                    matches!(ce, OngoingEffect::DoubleDamageTaken { affected_cards, except_strikes }
                        if affected_cards.matches(card_id, &snapshot) && !(*except_strikes && damage.is_strike))
                });
                let multiplier: u16 = if takes_double_damage { 2 } else { 1 };
                let adjusted_damage = damage * multiplier;
                let mut effects = if snapshot.animated_unit_base(card_id).is_some()
                    && !snapshot.get_card(card_id).is_unit()
                {
                    let dealer = snapshot.get_card(from);
                    let has_lethal_target = snapshot
                        .get_card(card_id)
                        .has_ability(&snapshot, &Ability::LethalTarget);
                    let reduced_damage = snapshot
                        .active_continuous_effects()
                        .into_iter()
                        .filter_map(|ce| match ce {
                            OngoingEffect::ReduceDamageTaken {
                                amount,
                                affected_cards,
                            } if affected_cards.matches(card_id, &snapshot) => Some(amount),
                            _ => None,
                        })
                        .fold(adjusted_damage.amount, |remaining, amount| {
                            remaining.saturating_sub(*amount)
                        });
                    let toughness = snapshot
                        .get_card(card_id)
                        .get_toughness(&snapshot)
                        .unwrap_or(0);
                    let new_damage = {
                        let base = state
                            .animated_unit_base_mut(card_id)
                            .ok_or(anyhow::anyhow!("animated card has no unit base"))?;
                        base.damage += reduced_damage;
                        base.damage
                    };

                    let mut effects = vec![];
                    let killer_id = if dealer.is_magic() {
                        state.find_caster(from).expect("magic to have a caster")
                    } else {
                        *from
                    };
                    if reduced_damage > 0
                        && (new_damage >= toughness
                            || adjusted_damage.is_lethal
                            || dealer.has_ability(&snapshot, &Ability::Lethal)
                            || has_lethal_target)
                    {
                        effects.push(Effect::KillMinion {
                            card_id: *card_id,
                            killer_id,
                            from_attack: adjusted_damage.is_attack,
                        });
                    }

                    if dealer.has_ability(&snapshot, &Ability::Lifesteal) {
                        let controller_id = dealer.get_controller_id(&snapshot);
                        if let Ok(avatar_id) = snapshot.get_player_avatar_id(&controller_id) {
                            let heal = dealer.get_power(&snapshot)?.unwrap_or(0);
                            if heal > 0 {
                                effects.push(Effect::Heal {
                                    card_id: avatar_id,
                                    amount: heal,
                                });
                            }
                        }
                    }

                    effects
                } else {
                    let card = state.get_card_mut(card_id);
                    card.base_take_damage(&snapshot, from, adjusted_damage)?
                };
                if damage.is_strike {
                    let dealer = snapshot.get_card(from);
                    if dealer.has_ability(&snapshot, &Ability::SplashDamage) {
                        let target = snapshot.get_card(card_id);
                        let dealer_controller = dealer.get_controller_id(&snapshot);
                        effects.extend(
                            CardQuery::new()
                                .units()
                                .in_zone(target.get_zone())
                                .id_not(card_id)
                                .all(&snapshot)
                                .into_iter()
                                .filter(|id| {
                                    snapshot.get_card(id).get_controller_id(&snapshot)
                                        != dealer_controller
                                })
                                .map(|id| Effect::TakeDamage {
                                    card_id: id,
                                    from: *from,
                                    damage: Damage::basic(damage.amount),
                                }),
                        );
                    }
                }
                state.queue(effects);
            }
            Effect::BanishCard { card_id, .. } => {
                let is_token = state.get_card(card_id).is_token();
                let card = state.get_card_mut(card_id);
                card.set_bearer_id(None);
                card.set_zone(Zone::Banish);

                let borne_cards: Vec<CardId> = state
                    .cards
                    .values()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| {
                        c.get_bearer_id()
                            .ok()
                            .flatten()
                            .filter(|bearer_id| bearer_id == card_id)
                            .map(|_| *c.get_id())
                    })
                    .collect();
                for borne_card_id in borne_cards {
                    state.get_card_mut(&borne_card_id).set_bearer_id(None);
                }

                // Tokens cease to exist when they leave the realm.
                if is_token {
                    state.queue_one(Effect::RemoveCardFromGame { card_id: *card_id });
                }
            }
            Effect::KillMinion { card_id, .. } => {
                state.queue_one(Effect::BuryCard { card_id: *card_id });
            }
            Effect::BuryCard { card_id, .. } => {
                // Deathrite fires BEFORE the card moves to the cemetery so that triggers
                // which care about the card's current zone (e.g. summoning a token in its
                // place) see it still in the realm.
                let card = state.get_card(card_id);
                let original_zone = card.get_zone().clone();
                let is_token = card.is_token();
                let is_site = card.is_site();
                let controller_id = card.get_controller_id(state);
                let effects = card.deathrite(state, &original_zone);
                state.queue(effects);

                // Effects are drained from the back of the queue, so enqueue removal before any
                // cleanup effects that still need to reference the token.
                if is_token {
                    state.queue_one(Effect::RemoveCardFromGame { card_id: *card_id });
                }

                state.queue_one(Effect::SetBearer {
                    card_id: *card_id,
                    bearer_id: None,
                });

                // All destroyed sites get replaced by a rubble, even other rubbles.
                if is_site && original_zone.is_in_play() {
                    state.queue_one(Effect::SummonToken {
                        player_id: controller_id,
                        token_type: TokenType::Rubble,
                        zone: original_zone.clone(),
                    });
                }

                state.get_card_mut(card_id).set_zone(Zone::Cemetery);

                let borne_cards = CardQuery::new().carried_by(card_id).all(state);
                for borne_card_id in borne_cards {
                    state.get_card_mut(&borne_card_id).set_bearer_id(None);
                }
            }
            Effect::Animate {
                card_id,
                unit_base,
                expires_on_effect,
            } => {
                state
                    .temporary_effects_mut()
                    .push(TemporaryEffect::Animate {
                        card_id: *card_id,
                        unit_base: unit_base.clone(),
                        expires_on_effect: expires_on_effect.clone(),
                    });
                state.queue(location_survival_effects_for_realm(state));
            }
            Effect::AddCounter {
                card_id, counter, ..
            } => {
                let card = state.get_card_mut(card_id);
                if card.is_unit() {
                    let base = card
                        .get_unit_base_mut()
                        .ok_or(anyhow::anyhow!("card has no unit base"))?;
                    base.power_counters.push(counter.clone());
                } else if let Some(base) = state.animated_unit_base_mut(card_id) {
                    base.power_counters.push(counter.clone());
                }
            }
            Effect::AddAbilityCounter {
                card_id, counter, ..
            } => {
                let card = state.get_card_mut(card_id);
                if card.is_unit() {
                    let base = card
                        .get_unit_base_mut()
                        .ok_or(anyhow::anyhow!("card has no unit base"))?;
                    base.ability_counters.push(counter.clone());
                } else if let Some(base) = state.animated_unit_base_mut(card_id) {
                    base.ability_counters.push(counter.clone());
                }
            }
            Effect::AddStatusCounter {
                card_id, counter, ..
            } => {
                let card = state.get_card_mut(card_id);
                card.get_base_mut().status_counters.push(counter.clone());
            }
            Effect::SetCardData { card_id, data, .. } => {
                let card = state.get_card_mut(card_id);
                card.set_data(data)?;
                state.queue(location_survival_effects_for_realm(state));
            }
            Effect::TeleportCard {
                player_id,
                card_id,
                to_location,
                ..
            } => {
                let card = state.get_card(card_id);
                state.queue_one(Effect::MoveCard {
                    player_id: *player_id,
                    card_id: *card_id,
                    from: card
                        .get_zone()
                        .clone()
                        .into_location()
                        .expect("MoveCard source must be a location"),
                    to: LocationQuery::from_location(to_location.clone()),
                    tap: false,
                    through_path: None,
                });
            }
            Effect::RearrangeDeck { spells, sites, .. } => {
                let current_player = state.current_player();
                let deck = state
                    .decks
                    .get_mut(&current_player)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                deck.spells = spells.clone();
                deck.sites = sites.clone();
            }
            Effect::SetCardRegion {
                card_id,
                destination: region,
                tap,
            } => {
                let card = state.get_card_mut(card_id);
                card.set_region(region.clone());
                if *tap {
                    card.set_tapped(true);
                }

                let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                for carried_card_id in &carried_cards {
                    state.get_card_mut(carried_card_id).set_region(region.clone());
                }

                state.queue(location_survival_effects_for_cards(
                    state,
                    std::iter::once(*card_id).chain(carried_cards),
                ));
            }
            Effect::SetBearer { card_id, bearer_id } => {
                if let Some(target) = state.cards.get_mut(card_id) {
                    target.set_bearer_id(*bearer_id);
                    state.invalidate_runtime_caches();
                }
            }
            Effect::ShuffleDeck { player_id } => {
                let deck = state
                    .decks
                    .get_mut(player_id)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                deck.shuffle();
            }
            Effect::DiscardCard { player_id, card_id } => {
                let card = state.get_card_mut(card_id);
                let original_zone = card.get_zone().clone();
                if card.get_owner_id() != player_id {
                    return Ok(());
                }
                card.set_zone(Zone::Cemetery);

                if original_zone == Zone::Spellbook {
                    state
                        .get_player_deck_mut(player_id)?
                        .spells
                        .retain(|id| id != card_id);
                }

                if original_zone == Zone::Atlasbook {
                    state
                        .get_player_deck_mut(player_id)?
                        .sites
                        .retain(|id| id != card_id);
                }
            }
            Effect::SetController { card_id, player_id } => {
                let card = state.get_card_mut(card_id);
                card.get_base_mut().controller_id = *player_id;
            }
            Effect::MakeCardCopyOf {
                card_id,
                copy_source_id,
            } => {
                let original_base = state.get_card(card_id).get_base().clone();
                let mut replacement = state
                    .cards
                    .get(copy_source_id)
                    .ok_or(anyhow::anyhow!("copy source card not found"))?
                    .clone();
                let replacement_base = replacement.get_base_mut();
                replacement_base.id = original_base.id;
                replacement_base.owner_id = original_base.owner_id;
                replacement_base.controller_id = original_base.controller_id;
                replacement_base.zone = original_base.zone;
                replacement_base.bearer = original_base.bearer;
                replacement_base.is_token = original_base.is_token;
                state.cards.insert(*card_id, replacement);
                state.invalidate_runtime_caches();
            }
            Effect::CopyMagic {
                source_id: _,
                player_id,
                card_id,
                caster_id,
            } => {
                let mut copy = state
                    .cards
                    .get(card_id)
                    .ok_or(anyhow::anyhow!("magic card to copy not found"))?
                    .clone();
                copy.get_base_mut().id = uuid::Uuid::new_v4();
                copy.get_base_mut().owner_id = *player_id;
                copy.get_base_mut().controller_id = *player_id;
                copy.get_base_mut().is_token = true;
                let effects = copy
                    .get_magic()
                    .ok_or(anyhow::anyhow!("magic card does not implement magic"))?
                    .resolve_magic(state, caster_id, Cost::ZERO.clone())
                    .await?;
                state.queue(effects);
            }
            Effect::CopyArtifact {
                player_id,
                artifact_id,
                bearer_id,
                caster_id,
            } => {
                let mut copy = state
                    .cards
                    .get(artifact_id)
                    .ok_or(anyhow::anyhow!("artifact card to copy not found"))?
                    .clone();
                let copy_base = copy.get_base_mut();
                copy_base.id = uuid::Uuid::new_v4();
                copy_base.owner_id = *player_id;
                copy_base.controller_id = *player_id;
                copy_base.is_token = true;
                copy.set_bearer_id(*bearer_id);
                let copy_id = *copy.get_id();
                state.cards.insert(*copy.get_id(), copy);
                state.invalidate_runtime_caches();

                if bearer_id.is_none() {
                    let copy = state.get_card(&copy_id);
                    let effects = copy.play_mechanic(state, player_id, caster_id).await?;
                    state.queue(effects);
                } else {
                    let copy = state.get_card(&copy_id);
                    let effects = copy.genesis(state).await?;
                    state.queue(effects);
                }
            }
            Effect::SummonCopy {
                card_name,
                player_id,
                zone,
            } => {
                let mut copy = crate::card::from_name(card_name, player_id);
                copy.get_base_mut().is_token = true;

                let has_charge = copy.has_ability(state, &Ability::Charge);
                let copy_id = *copy.get_id();
                state.cards.insert(copy_id, copy);
                state.invalidate_runtime_caches();

                let card = state.get_card_mut(&copy_id);
                card.set_zone(zone.clone());
                if !has_charge {
                    card.add_status(CardStatus::SummoningSickness);
                }

                state
                    .add_passive_ongoing_effects_for_source(&copy_id)
                    .await?;

                crate::game::force_sync_all(state).await?;

                let card = state.get_card(&copy_id);
                let mut effects: Vec<Effect> = vec![];
                effects.extend(card.genesis(state).await?);
                effects.push(Effect::BanishCard { card_id: copy_id });
                state.queue(effects);
            }
            Effect::RemoveCardFromGame { card_id } => {
                state.remove_ongoing_effects_from_source(card_id);
                if let Some(card) = state.cards.remove(card_id) {
                    state.removed_cards.insert(*card_id, card);
                }
                state.invalidate_runtime_caches();
            }
        }

        state.invalidate_runtime_caches();
        let area_effects: Vec<Effect> = state
            .cards
            .values()
            .filter(|c| can_use_special_abilities(state, c.get_id()))
            .filter_map(|c| c.area_effects(state).ok())
            .flatten()
            .collect();
        state.queue(area_effects);

        self.expire_counters(state).await?;
        EffectLifecycle::after_resolved_effect(state, self).await?;

        crate::game::force_sync_all(state).await?;
        state.invalidate_runtime_caches();

        Ok(())
    }
}
