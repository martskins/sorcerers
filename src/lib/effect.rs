use crate::{
    card::{Ability, Card, FootSoldier, Frog, Region, Rubble, UnitBase, Zone},
    game::{BaseAction, Direction, PlayerAction, PlayerId, SoundEffect, pick_card, pick_option},
    networking::message::ServerMessage,
    query::{EffectQuery, QueryCache, ZoneQuery},
    state::{CardQuery, ContinuousEffect, DeferredEffect, Phase, State, TemporaryEffect},
};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AbilityCounter {
    pub id: uuid::Uuid,
    pub ability: Ability,
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

#[derive(Debug)]
pub enum TokenType {
    Rubble,
    FootSoldier,
    Frog,
}

#[derive(Debug)]
pub enum Effect {
    PlayerLost {
        player_id: PlayerId,
    },
    SummonToken {
        player_id: PlayerId,
        token_type: TokenType,
        zone: Zone,
    },
    Heal {
        card_id: uuid::Uuid,
        amount: u16,
    },
    ShootProjectile {
        id: uuid::Uuid,
        player_id: PlayerId,
        shooter: uuid::Uuid,
        from_zone: Zone,
        direction: Direction,
        damage: u16,
        piercing: bool,
        splash_damage: Option<u16>,
    },
    RemoveAbility {
        card_id: uuid::Uuid,
        modifier: Ability,
    },
    AddAbilityCounter {
        card_id: uuid::Uuid,
        counter: AbilityCounter,
    },
    AddCounter {
        card_id: uuid::Uuid,
        counter: Counter,
    },
    SetCardRegion {
        card_id: uuid::Uuid,
        region: Region,
        tap: bool,
    },
    SetCardZone {
        card_id: uuid::Uuid,
        zone: Zone,
    },
    DiscardCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
    },
    MoveCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        from: Zone,
        to: ZoneQuery,
        tap: bool,
        region: Region,
        through_path: Option<Vec<Zone>>,
    },
    DrawSite {
        player_id: PlayerId,
        count: u8,
    },
    DrawSpell {
        player_id: PlayerId,
        count: u8,
    },
    DrawCard {
        player_id: PlayerId,
        count: u8,
    },
    PlayMagic {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        caster_id: uuid::Uuid,
        from: Zone,
    },
    PlayCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        zone: ZoneQuery,
    },
    SummonCards {
        cards: Vec<(PlayerId, uuid::Uuid, Zone)>,
    },
    SummonCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        zone: Zone,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
    UntapCard {
        card_id: uuid::Uuid,
    },
    EndTurn {
        player_id: PlayerId,
    },
    StartTurn {
        player_id: PlayerId,
    },
    ConsumeMana {
        player_id: PlayerId,
        mana: u8,
    },
    AddMana {
        player_id: PlayerId,
        mana: u8,
    },
    RangedStrike {
        striker_id: uuid::Uuid,
        target_id: uuid::Uuid,
    },
    Strike {
        striker_id: uuid::Uuid,
        target_id: uuid::Uuid,
    },
    Attack {
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
    },
    TakeDamage {
        card_id: uuid::Uuid,
        from: uuid::Uuid,
        damage: u16,
        is_strike: bool,
    },
    BanishCard {
        card_id: uuid::Uuid,
        from: Zone,
    },
    KillMinion {
        card_id: uuid::Uuid,
        killer_id: uuid::Uuid,
    },
    BuryCard {
        card_id: uuid::Uuid,
    },
    SetCardData {
        card_id: uuid::Uuid,
        data: Box<dyn std::any::Any + Send + Sync>,
    },
    TeleportCard {
        player_id: PlayerId,
        card_id: uuid::Uuid,
        to_zone: Zone,
    },
    DealDamageAllUnitsInZone {
        player_id: PlayerId,
        zone: ZoneQuery,
        from: uuid::Uuid,
        damage: u16,
    },
    DealDamageToTarget {
        player_id: PlayerId,
        query: CardQuery,
        from: uuid::Uuid,
        damage: u16,
    },
    RearrangeDeck {
        spells: Vec<uuid::Uuid>,
        sites: Vec<uuid::Uuid>,
    },
    AddDeferredEffect {
        effect: DeferredEffect,
    },
    AddTemporaryEffect {
        effect: TemporaryEffect,
    },
    SetBearer {
        card_id: uuid::Uuid,
        bearer_id: Option<uuid::Uuid>,
    },
    ShuffleDeck {
        player_id: PlayerId,
    },
    SetController {
        card_id: uuid::Uuid,
        player_id: PlayerId,
    },
    /// Creates a token copy of the named card for the given player and summons it in the target
    /// zone. The copy triggers its Genesis, then is automatically banished afterwards.
    SummonCopy {
        card_name: String,
        player_id: PlayerId,
        zone: Zone,
    },
}

fn player_name<'a>(player_id: &PlayerId, state: &'a State) -> &'a str {
    match state.players.iter().find(|p| &p.id == player_id) {
        Some(player) => &player.name,
        None => "Unknown Player",
    }
}

impl Effect {
    pub async fn affected_cards(&self) -> Option<Vec<uuid::Uuid>> {
        match self {
            Effect::ShootProjectile { id, .. } => QueryCache::effect_targets(id).await,
            _ => None,
        }
    }

    pub fn take_damage(card_id: &uuid::Uuid, from: &uuid::Uuid, damage: u16) -> Self {
        Effect::TakeDamage {
            card_id: card_id.clone(),
            from: from.clone(),
            damage,
            is_strike: false,
        }
    }

    pub fn strike_damage(card_id: &uuid::Uuid, from: &uuid::Uuid, damage: u16) -> Self {
        Effect::TakeDamage {
            card_id: card_id.clone(),
            from: from.clone(),
            damage,
            is_strike: true,
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
            Effect::PlayerLost { player_id } => Some(player_id),
            Effect::SummonToken { player_id, .. } => Some(player_id),
            Effect::Heal { card_id, .. } => Some(card_id),
            Effect::ShootProjectile { player_id, .. } => Some(player_id),
            Effect::RemoveAbility { card_id, .. } => Some(card_id),
            Effect::AddAbilityCounter { card_id, .. } => Some(card_id),
            Effect::AddCounter { card_id, .. } => Some(card_id),
            Effect::SetCardRegion { card_id, .. } => Some(card_id),
            Effect::SetCardZone { card_id, .. } => Some(card_id),
            Effect::MoveCard { card_id, .. } => Some(card_id),
            Effect::DiscardCard { card_id, .. } => Some(card_id),
            Effect::DrawSite { player_id, .. } => Some(player_id),
            Effect::DrawSpell { player_id, .. } => Some(player_id),
            Effect::DrawCard { player_id, .. } => Some(player_id),
            Effect::PlayMagic { card_id, .. } => Some(card_id),
            Effect::PlayCard { card_id, .. } => Some(card_id),
            Effect::SummonCard { card_id, .. } => Some(card_id),
            Effect::SummonCards { .. } => None,
            Effect::TapCard { card_id } => Some(card_id),
            Effect::UntapCard { card_id } => Some(card_id),
            Effect::EndTurn { player_id } => Some(player_id),
            Effect::StartTurn { player_id } => Some(player_id),
            Effect::ConsumeMana { player_id, .. } => Some(player_id),
            Effect::AddMana { player_id, .. } => Some(player_id),
            Effect::Strike { striker_id, .. } => Some(striker_id),
            Effect::RangedStrike { striker_id, .. } => Some(striker_id),
            Effect::Attack { attacker_id, .. } => Some(attacker_id),
            Effect::TakeDamage { card_id, .. } => Some(card_id),
            Effect::BanishCard { card_id, .. } => Some(card_id),
            Effect::KillMinion { card_id, .. } => Some(card_id),
            Effect::BuryCard { card_id } => Some(card_id),
            Effect::SetCardData { card_id, .. } => Some(card_id),
            Effect::TeleportCard { player_id, .. } => Some(player_id),
            Effect::DealDamageAllUnitsInZone { from, .. } => Some(from),
            Effect::DealDamageToTarget { from, .. } => Some(from),
            Effect::RearrangeDeck { .. } => None,
            Effect::AddDeferredEffect { .. } => None,
            Effect::AddTemporaryEffect { .. } => None,
            Effect::SetBearer { card_id, .. } => Some(card_id),
            Effect::ShuffleDeck { .. } => None,
            Effect::SetController { card_id, .. } => Some(card_id),
            Effect::SummonCopy { player_id, .. } => Some(player_id),
        }
    }

    /// Returns the card ID if this effect represents a card being played from hand
    /// (PlayCard or PlayMagic), so clients can display the card face to all players.
    pub fn played_card_id(&self) -> Option<uuid::Uuid> {
        match self {
            Effect::PlayCard { card_id, .. } => Some(*card_id),
            Effect::PlayMagic { card_id, .. } => Some(*card_id),
            _ => None,
        }
    }

    pub async fn description(&self, state: &State) -> anyhow::Result<Option<String>> {
        let desc = match self {
            Effect::PlayerLost { player_id } => Some(format!(
                "{} has lost the game",
                player_name(player_id, state)
            )),
            Effect::SetCardRegion {
                card_id, region, ..
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
            Effect::ShootProjectile {
                player_id,
                shooter,
                damage,
                direction,
                ..
            } => {
                let shooter_card = state.get_card(shooter).get_name();
                Some(format!(
                    "{} shoots a projectile for {} damage from {} in direction {}",
                    player_name(player_id, state),
                    damage,
                    shooter_card,
                    direction.get_name()
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
                if card.get_zone() == from {
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
                        to.resolve(player_id, state).await?,
                    )),
                }
            }
            Effect::DrawCard { player_id, count } => {
                if *count == 0 {
                    return Ok(None);
                }
                let cards = if *count == 1 { "card" } else { "cards" };
                Some(format!(
                    "{} draws {} {}",
                    player_name(player_id, state),
                    count,
                    cards
                ))
            }
            Effect::DrawSite {
                player_id, count, ..
            } => {
                if *count == 0 {
                    return Ok(None);
                }

                let sites = if *count == 1 { "site" } else { "sites" };
                Some(format!(
                    "{} draws {} {}",
                    player_name(player_id, state),
                    count,
                    sites
                ))
            }
            Effect::DrawSpell {
                player_id, count, ..
            } => {
                if *count == 0 {
                    return Ok(None);
                }

                let spells = if *count == 1 { "spell" } else { "spells" };
                Some(format!(
                    "{} draws {} {}",
                    player_name(player_id, state),
                    count,
                    spells
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
                    zone.resolve(player_id, state).await?,
                ))
            }
            Effect::SummonCards { cards } => {
                if cards.is_empty() {
                    None
                } else {
                    let parts: Vec<String> = cards
                        .iter()
                        .map(|(player_id, card_id, zone)| {
                            format!(
                                "{} summons {} in {}",
                                player_name(player_id, state),
                                state.get_card(card_id).get_name(),
                                zone
                            )
                        })
                        .collect();
                    Some(parts.join("; "))
                }
            }
            Effect::SummonCard {
                player_id,
                card_id,
                zone,
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!(
                    "{} summons {} in {}",
                    player_name(player_id, state),
                    card,
                    zone
                ))
            }
            Effect::TapCard { .. } => None,
            Effect::UntapCard { .. } => None,
            Effect::EndTurn { player_id, .. } => {
                Some(format!("{} passes the turn", player_name(player_id, state)))
            }
            Effect::StartTurn { player_id } => Some(format!(
                "--- {}'s turn begins ---",
                player_name(player_id, state)
            )),
            Effect::ConsumeMana { .. } => None,
            Effect::AddMana { .. } => None,
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
                    defender, damage, attacker
                ))
            }
            Effect::KillMinion { card_id, killer_id } => {
                let card = state.get_card(card_id);
                let killer = state.get_card(killer_id);
                Some(format!("{} kills {}", card.get_name(), killer.get_name()))
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
            Effect::SetCardData { .. } => None,
            Effect::RangedStrike {
                striker_id,
                target_id,
            } => Some(format!(
                "{} ranged strikes {} with {}",
                player_name(&state.get_card(striker_id).get_controller_id(state), state),
                state.get_card(target_id).get_name(),
                state.get_card(striker_id).get_name(),
            )),
            Effect::DealDamageToTarget { .. } => None,
            Effect::DealDamageAllUnitsInZone {
                player_id,
                zone,
                from,
                damage,
            } => {
                let source = state.get_card(from).get_name();
                let zone_name = zone.resolve(player_id, state).await?;
                Some(format!(
                    "{} deals {} damage to all units in {}",
                    source, damage, zone_name
                ))
            }
            Effect::TeleportCard {
                player_id,
                card_id,
                to_zone,
            } => {
                let card = state.get_card(card_id).get_name();
                Some(format!(
                    "{} teleports {} to {}",
                    player_name(player_id, state),
                    card,
                    to_zone
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
        };

        Ok(desc)
    }

    async fn process_deferred_effects(
        &self,
        state: &mut State,
        effect: &Effect,
    ) -> anyhow::Result<()> {
        let mut effects_to_remove = vec![];
        for (idx, de) in state.deferred_effects.iter().enumerate() {
            if de.trigger_on_effect.matches(effect, state).await? {
                if let Some(source_id) = effect.source_id() {
                    let effects = (de.on_effect)(state, source_id, effect).await?;
                    state
                        .effects
                        .extend(effects.into_iter().map(std::sync::Arc::new));
                }

                if !de.multitrigger {
                    effects_to_remove.push(idx);
                }
            } else {
                // If the effect was not triggered, check whether it needs to expire.
                if let Some(expires_on_effect) = &de.expires_on_effect {
                    if expires_on_effect.matches(effect, state).await? {
                        effects_to_remove.push(idx);
                    }
                }
            }
        }

        // Reverse the list of processed effects indexes so that we remove them from the deferred
        // effects list starting from the end, to avoid messing up the indexes of unprocessed
        // effects.
        effects_to_remove.reverse();
        for idx in effects_to_remove {
            state.deferred_effects.remove(idx);
        }

        Ok(())
    }

    async fn expire_temporary_effects(
        &self,
        state: &mut State,
        effect: &Effect,
    ) -> anyhow::Result<()> {
        let snapshot = state.snapshot();
        let mut retained_effects = vec![];
        for te in &state.temporary_effects {
            let should_retain = match te.expires_on_effect() {
                Some(expiry_effect) => !expiry_effect.matches(effect, &snapshot).await?,
                None => true,
            };

            if should_retain {
                retained_effects.push(te.clone());
            }
        }

        state.temporary_effects = retained_effects;
        Ok(())
    }

    async fn expire_counters(&self, state: &mut State) -> anyhow::Result<()> {
        let modified_cards: Vec<&Box<dyn Card>> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| {
                c.get_unit_base()
                    .expect("unit to have a unit base component")
                    .modifier_counters
                    .len()
                    > 0
            })
            .collect();
        let mut card_modifiers_to_remove: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = vec![];
        for card in modified_cards {
            let mut to_remove: Vec<uuid::Uuid> = vec![];
            for counter in &card
                .get_unit_base()
                .unwrap_or(&UnitBase::default())
                .modifier_counters
            {
                if let Some(effect_query) = &counter.expires_on_effect {
                    if effect_query.matches(self, state).await? {
                        to_remove.push(counter.id);
                    }
                }
            }

            if !to_remove.is_empty() {
                card_modifiers_to_remove.push((card.get_id().clone(), to_remove));
            }
        }

        for (card_id, to_remove) in card_modifiers_to_remove {
            let card_mut = state.get_card_mut(&card_id);
            for counter_id in to_remove {
                card_mut.remove_modifier_counter(&counter_id);
            }
        }

        let cards_with_counters: Vec<&Box<dyn Card>> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| {
                c.get_unit_base()
                    .unwrap_or(&UnitBase::default())
                    .power_counters
                    .len()
                    > 0
            })
            .collect();
        let mut card_counters_to_remove: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = vec![];
        for card in cards_with_counters {
            let mut to_remove: Vec<uuid::Uuid> = vec![];
            for counter in &card
                .get_unit_base()
                .unwrap_or(&UnitBase::default())
                .power_counters
            {
                if let Some(effect_query) = &counter.expires_on_effect {
                    if effect_query.matches(self, state).await? {
                        to_remove.push(counter.id);
                    }
                }
            }

            if !to_remove.is_empty() {
                card_counters_to_remove.push((card.get_id().clone(), to_remove));
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
        let mut effects: Vec<Effect> = vec![];
        for card in &state.cards {
            let replace_effects = card.replace_effect(state, self).await?.unwrap_or_default();
            effects.extend(replace_effects);
        }

        if let Some(replaced_effects) = state.replace_effect(self).await? {
            effects = replaced_effects;
        }

        if !effects.is_empty() {
            state.queue(effects);
            return Ok(());
        }

        match self {
            Effect::PlayerLost { player_id } => {
                state.loosers.insert(player_id.clone());
            }
            Effect::AddDeferredEffect { effect, .. } => {
                state.deferred_effects.push(effect.clone());
            }
            Effect::AddTemporaryEffect { effect } => {
                state.temporary_effects.push(effect.clone());
            }
            Effect::SetCardZone { card_id, zone } => {
                let card = state.get_card_mut(card_id);
                card.set_zone(zone.clone());
            }
            Effect::SummonToken {
                player_id,
                token_type,
                zone,
            } => {
                let mut token: Box<dyn Card> = match token_type {
                    TokenType::Rubble => Box::new(Rubble::new(player_id.clone())),
                    TokenType::FootSoldier => Box::new(FootSoldier::new(player_id.clone())),
                    TokenType::Frog => Box::new(Frog::new(player_id.clone())),
                };
                token.set_zone(zone.clone());
                state.cards.push(token);
            }
            Effect::Heal { card_id, amount } => {
                let card = state.get_card_mut(card_id);
                let unit_base = card
                    .get_unit_base_mut()
                    .ok_or(anyhow::anyhow!("card has no unit base"))?;
                unit_base.damage = unit_base.damage.saturating_sub(*amount);
            }
            Effect::RemoveAbility { card_id, modifier } => {
                let card = state.get_card_mut(card_id);
                card.remove_modifier(modifier);
            }
            Effect::ShootProjectile {
                id,
                player_id,
                shooter,
                from_zone,
                direction,
                damage,
                piercing,
                splash_damage,
                ..
            } => {
                let mut effects = vec![];
                let mut next_zone = from_zone.zone_in_direction(direction, 1);
                while let Some(zone) = next_zone {
                    let picked_unit_id = match self.affected_cards().await {
                        Some(affected_cards) => affected_cards.first().cloned(),
                        None => {
                            let units = state
                                .get_units_in_zone(&zone)
                                .iter()
                                .filter(|c| c.can_be_targetted_by(state, player_id))
                                .map(|c| c.get_id().clone())
                                .collect::<Vec<_>>();
                            match units.len() {
                                0 => None,
                                1 => Some(units[0].clone()),
                                _ => {
                                    let prompt = "Pick a unit to shoot";
                                    let picked_unit_id =
                                        pick_card(player_id, &units, state, prompt).await?;
                                    QueryCache::store_effect_targets(
                                        state.game_id.clone(),
                                        id.clone(),
                                        vec![picked_unit_id.clone()],
                                    )
                                    .await;
                                    Some(picked_unit_id)
                                }
                            }
                        }
                    };

                    if let Some(picked_unit_id) = picked_unit_id {
                        effects.push(Effect::TakeDamage {
                            card_id: picked_unit_id.clone(),
                            from: shooter.clone(),
                            damage: *damage,
                            is_strike: false,
                        });
                        if let Some(splash_damage) = splash_damage {
                            let splash_effects = state
                                .get_units_in_zone(&zone)
                                .iter()
                                .filter(|c| c.get_id() != &picked_unit_id)
                                .map(|c| Effect::TakeDamage {
                                    card_id: c.get_id().clone(),
                                    from: shooter.clone(),
                                    damage: *splash_damage,
                                    is_strike: false,
                                })
                                .collect::<Vec<_>>();
                            effects.extend(splash_effects);
                        }

                        if !piercing {
                            break;
                        }
                    }

                    next_zone = zone.zone_in_direction(direction, 1);
                }

                for effect in effects {
                    state.effects.push_back(effect.into());
                }
            }
            Effect::MoveCard {
                player_id,
                card_id,
                from,
                to,
                tap,
                through_path,
                ..
            } => {
                let card = state.get_card(card_id);
                // Skip the move if the card is no longer in the same zone as it was originally.
                if card.get_zone() != from {
                    return Ok(());
                }

                match through_path {
                    Some(path) => {
                        for zone in path {
                            let snapshot = state.snapshot();
                            let zone = ZoneQuery::from_zone(zone.clone())
                                .resolve(player_id, state)
                                .await?;
                            let card = state.get_card_mut(card_id);
                            card.set_zone(zone.clone());
                            if *tap {
                                card.set_tapped(true);
                            }

                            let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                            for cid in carried_cards {
                                let carried_card = state.get_card_mut(&cid);
                                carried_card.set_zone(zone.clone());
                                if *tap {
                                    carried_card.set_tapped(true);
                                }
                            }

                            let card = state.get_card(card_id);
                            let mut effects = card.on_move(&snapshot, path).await?;
                            effects.extend(card.on_visit_zone(&snapshot, &zone).await?);
                            if let Some(site) = zone.get_site(state) {
                                effects.extend(site.on_card_enter(state, card_id));
                            }

                            state.queue(effects);
                        }
                    }
                    None => {
                        let snapshot = state.snapshot();
                        let zone = to.resolve(player_id, state).await?;
                        let card = state.get_card_mut(card_id);
                        card.set_zone(zone.clone());
                        if *tap {
                            card.set_tapped(true);
                        }

                        let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                        for cid in carried_cards {
                            let carried_card = state.get_card_mut(&cid);
                            carried_card.set_zone(zone.clone());
                            if *tap {
                                carried_card.set_tapped(true);
                            }
                        }

                        let card = state.get_card(card_id);
                        let path = vec![from.clone(), zone.clone()];
                        let mut effects = card.on_move(&snapshot, &path).await?;
                        effects.extend(card.on_visit_zone(&snapshot, &zone).await?);
                        if let Some(site) = zone.get_site(state) {
                            effects.extend(site.on_card_enter(state, card_id));
                        }

                        state.queue(effects);
                    }
                }
            }
            Effect::DrawSite {
                player_id, count, ..
            } => {
                let deck = state
                    .decks
                    .get_mut(player_id)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                for _ in 0..*count {
                    if let Some(card_id) = deck.sites.pop() {
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == &card_id)
                            .expect("to find drawn card")
                            .set_zone(Zone::Hand);
                    }
                }
            }
            Effect::DrawSpell {
                player_id, count, ..
            } => {
                let deck = state
                    .decks
                    .get_mut(player_id)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                for _ in 0..*count {
                    if let Some(card_id) = deck.spells.pop() {
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == &card_id)
                            .expect("to find drawn card")
                            .set_zone(Zone::Hand);
                    }
                }
            }
            Effect::DrawCard {
                player_id, count, ..
            } => {
                for _ in 0..*count {
                    let options: Vec<BaseAction> =
                        vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                    let option_labels = options
                        .iter()
                        .map(|a| a.get_name().to_string())
                        .collect::<Vec<_>>();
                    let picked_option_idx =
                        pick_option(player_id, &option_labels, state, "Draw a card", false).await?;
                    let effects = options[picked_option_idx]
                        .on_select(player_id, state)
                        .await?;
                    state.queue(effects);
                }
            }
            Effect::PlayMagic {
                card_id,
                player_id,
                caster_id,
                ..
            } => {
                let costs = state.get_effective_costs(card_id, None)?;
                let paid_cost = costs.pay(state, player_id).await?;

                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card.on_cast(&snapshot, caster_id, paid_cost).await?;

                // Set zone after on_cast so that the card is not in the cemetery during casting.
                card.set_zone(Zone::Cemetery);
                state.queue(effects);

                // Notify the Spellcaster unit that it cast a spell.
                let caster = state.get_card(caster_id);
                let spell_triggered = caster.on_cast_spell(state, card_id).await?;
                state.queue(spell_triggered);
            }
            Effect::PlayCard {
                card_id,
                player_id,
                zone,
                ..
            } => {
                let zone = zone.resolve(player_id, state).await?;
                let costs = state.get_effective_costs(card_id, Some(&zone))?;
                Box::pin(costs.pay(state, &player_id)).await?;
                let snapshot = state.snapshot();
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");

                // If playing a site and there is a rubble on that zone, remove it.
                if card.is_site() {
                    if let Some(site) = zone.get_site(&snapshot) {
                        if site.get_name() == Rubble::NAME {
                            state
                                .effects
                                .push_back(std::sync::Arc::new(Effect::BanishCard {
                                    card_id: site.get_id().clone(),
                                    from: zone.clone(),
                                }));
                        }
                    }
                }

                let cast_effects = card.on_summon(&snapshot)?;
                card.set_zone(zone.clone());
                if !card.has_ability(&snapshot, &Ability::Charge) {
                    card.add_modifier(Ability::SummoningSickness);
                }

                let mut effects = card.genesis(&snapshot).await?;
                effects.extend(card.on_visit_zone(&snapshot, &zone).await?);
                state.queue(effects);
                state.queue(cast_effects);
            }
            Effect::SummonCards { cards } => {
                for (_, card_id, zone) in cards {
                    let has_charge = state.get_card(card_id).has_ability(state, &Ability::Charge);
                    let card = state.get_card_mut(card_id);
                    card.set_zone(zone.clone());

                    if !has_charge {
                        card.add_modifier(Ability::SummoningSickness);
                    }
                }

                // Force sync after all cards have been put on their zones, so that players see them
                // on the board while resolving effects from on_summon, genesis and on_visit_zone.
                for player in &state.players {
                    crate::game::force_sync(&player.id, state).await?;
                }

                let mut effects = vec![];
                for (_, card_id, zone) in cards {
                    let card = state.get_card(card_id);
                    effects.extend(card.on_summon(state)?);
                    effects.extend(card.genesis(state).await?);
                    effects.extend(card.on_visit_zone(state, zone).await?);
                }

                state.queue(effects);
            }
            Effect::SummonCard { card_id, zone, .. } => {
                let has_charge = state.get_card(card_id).has_ability(state, &Ability::Charge);
                let card = state.get_card_mut(card_id);
                card.set_zone(zone.clone());

                if !has_charge {
                    card.add_modifier(Ability::SummoningSickness);
                }

                // Force sync after all cards have been put on their zones, so that players see them
                // on the board while resolving effects from on_summon, genesis and on_visit_zone.
                for player in &state.players {
                    crate::game::force_sync(&player.id, state).await?;
                }

                let mut effects = vec![];
                let card = state.get_card(card_id);
                effects.extend(card.on_summon(state)?);
                effects.extend(card.genesis(state).await?);
                effects.extend(card.on_visit_zone(state, zone).await?);

                state.queue(effects);
            }
            Effect::TapCard { card_id, .. } => {
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                card.set_tapped(true);

                let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                for cid in carried_cards {
                    state.get_card_mut(&cid).set_tapped(true);
                }
            }
            Effect::UntapCard { card_id, .. } => {
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                card.set_tapped(false);

                let carried_cards = CardQuery::new().carried_by(card_id).all(state);
                for cid in carried_cards {
                    state.get_card_mut(&cid).set_tapped(true);
                }
            }
            Effect::StartTurn { player_id, .. } => {
                let previous_player = state.current_player.clone();
                state
                    .get_sender()
                    .send(ServerMessage::Wait {
                        player_id: previous_player.clone(),
                        prompt: "Waiting for other player".to_string(),
                    })
                    .await?;

                state.current_player = player_id.clone();
                let cards = state
                    .cards
                    .iter_mut()
                    .filter(|c| c.get_owner_id() == &state.current_player);
                for card in cards {
                    card.set_tapped(false);
                    card.remove_modifier(&Ability::SummoningSickness);
                }

                let available_mana: u8 = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| match c.get_resource_provider() {
                        Some(rp) => Some(rp.provided_mana(state).expect("to get provided mana")),
                        None => None,
                    })
                    .sum();
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = available_mana;

                let mut all_effects: Vec<Effect> = vec![];
                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let effects = card.on_turn_start(state).await?;
                    all_effects.extend(effects);
                }
                state.queue(all_effects);

                let options: Vec<BaseAction> = vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                let option_labels: Vec<String> =
                    options.iter().map(|a| a.get_name().to_string()).collect();
                let prompt = "Start Turn: Pick card to draw";
                let picked_option_idx =
                    pick_option(player_id, &option_labels, state, prompt, false).await?;
                let effects = options[picked_option_idx]
                    .on_select(player_id, state)
                    .await?;
                state.queue(effects);

                state.turns += 1;
                state
                    .get_sender()
                    .send(ServerMessage::Resume {
                        player_id: previous_player,
                    })
                    .await?;
            }
            Effect::ConsumeMana {
                player_id, mana, ..
            } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = player_mana.saturating_sub(*mana);
            }
            Effect::EndTurn { player_id, .. } => {
                let mut all_effects: Vec<Effect> = vec![];
                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let effects = card.on_turn_end(state).await?;
                    all_effects.extend(effects);
                }
                state.queue(all_effects);

                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = 0;
                state.phase = Phase::Main;

                let cards = state.cards.iter_mut().filter(|c| c.is_unit());
                for card in cards {
                    if card.is_avatar() {
                        continue;
                    }

                    card.get_unit_base_mut()
                        .ok_or(anyhow::anyhow!("card has no unit base component"))?
                        .damage = 0;
                }

                let current_index = state
                    .players
                    .iter()
                    .position(|p| p.id == state.current_player)
                    .unwrap_or_default();
                let next_player = state
                    .players
                    .iter()
                    .cycle()
                    .skip(current_index + 1)
                    .next()
                    .ok_or(anyhow::anyhow!("No next player found"))?;

                // Push StartTurn to the front of the queue so all end of turn effects are resolved
                // first.
                state.queue_front(Effect::StartTurn {
                    player_id: next_player.id.clone(),
                });
            }
            Effect::AddMana {
                player_id, mana, ..
            } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana += mana;
            }
            Effect::Strike {
                striker_id,
                target_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(striker_id);
                let defender = state.get_card(target_id);
                let mut effects = vec![Effect::TakeDamage {
                    card_id: target_id.clone(),
                    from: striker_id.clone(),
                    damage: attacker
                        .get_power(&snapshot)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?,
                    is_strike: true,
                }];

                effects.extend(
                    defender
                        .on_defend(state, striker_id)?
                        .into_iter()
                        .map(|e| e.into()),
                );
                effects.reverse();
                state.queue(effects);
            }
            Effect::Attack {
                attacker_id,
                defender_id,
                ..
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let mut effects = vec![Effect::MoveCard {
                    player_id: attacker.get_controller_id(state).clone(),
                    card_id: attacker_id.clone(),
                    from: attacker.get_zone().clone(),
                    to: ZoneQuery::from_zone(defender.get_zone().clone()),
                    tap: true,
                    region: attacker.get_region(state).clone(),
                    through_path: None,
                }];

                let mut first_striker_id = attacker_id;
                let mut first_defender_id = defender_id;
                if defender.has_ability(state, &Ability::FirstStrike)
                    && !attacker.has_ability(&snapshot, &Ability::FirstStrike)
                {
                    first_striker_id = defender_id;
                    first_defender_id = attacker_id;
                }

                let first_striker = state.get_card(first_striker_id);
                let first_defender = state.get_card(first_defender_id);
                let strike_damage = first_striker
                    .get_power(&snapshot)?
                    .ok_or(anyhow::anyhow!("attacker has no power"))?;
                effects.push(Effect::TakeDamage {
                    card_id: first_defender_id.clone(),
                    from: first_striker_id.clone(),
                    damage: strike_damage,
                    is_strike: false,
                });

                let mut snapshot = state.snapshot();
                Box::pin(snapshot.apply_effects_without_log()).await?;

                let killed_defender =
                    state.get_card(first_defender_id).get_zone() == &Zone::Cemetery;
                if killed_defender
                    && first_striker.has_ability(state, &Ability::FirstStrike)
                    && !first_defender.has_ability(state, &Ability::FirstStrike)
                {
                    // If the first striker killed the defender before it could strike back, skip the defender's strike.
                    effects.reverse();
                    state.queue(effects);
                    return Ok(());
                }

                effects.extend(
                    defender
                        .on_defend(state, attacker_id)?
                        .into_iter()
                        .map(|e| e.into()),
                );
                effects.reverse();
                state.queue(effects);
            }
            Effect::DealDamageAllUnitsInZone {
                player_id,
                zone: query,
                from,
                damage,
            } => {
                let zone = query.resolve(player_id, state).await?;
                let units: Vec<uuid::Uuid> = zone
                    .get_units(state, None)
                    .iter()
                    .map(|c| c.get_id())
                    .cloned()
                    .collect();
                for unit_id in units {
                    state.queue_one(Effect::TakeDamage {
                        card_id: unit_id,
                        from: from.clone(),
                        damage: *damage,
                        is_strike: false,
                    });
                }
            }
            Effect::DealDamageToTarget {
                player_id,
                query,
                from,
                damage,
                ..
            } => {
                if let Some(target) = query.pick(player_id, state, false).await? {
                    state.queue_one(Effect::TakeDamage {
                        card_id: target,
                        from: from.clone(),
                        damage: *damage,
                        is_strike: false,
                    });
                }
            }
            Effect::TakeDamage {
                card_id,
                damage,
                from,
                is_strike,
            } => {
                let snapshot = state.snapshot();
                // Check if this card has DoubleDamageTaken applied to it.
                let takes_double_damage = snapshot.continuous_effects.iter().any(|ce| {
                    matches!(ce, ContinuousEffect::DoubleDamageTaken { affected_cards, except_strikes }
                        if affected_cards.matches(card_id, &snapshot) && !(*except_strikes && *is_strike))
                });
                let multiplier: u16 = if takes_double_damage { 2 } else { 1 };
                let card = state.get_card_mut(card_id);
                let effects = card.on_take_damage(&snapshot, from, *damage * multiplier)?;
                state.queue(effects);
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.get_card_mut(card_id);
                card.set_bearer_id(None);
                card.set_zone(Zone::Banish);

                let borne_cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| {
                        c.get_bearer_id()
                            .ok()
                            .flatten()
                            .filter(|bearer_id| bearer_id == card_id)
                            .map(|_| c.get_id().clone())
                    })
                    .collect();
                for borne_card_id in borne_cards {
                    state.get_card_mut(&borne_card_id).set_bearer_id(None);
                }
            }
            Effect::KillMinion { card_id, .. } => {
                state.queue_one(Effect::BuryCard {
                    card_id: card_id.clone(),
                });
            }
            Effect::BuryCard { card_id, .. } => {
                let card = state.get_card_mut(card_id);
                let original_zone = card.get_zone().clone();
                card.set_bearer_id(None);
                card.set_zone(Zone::Cemetery);

                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card.deathrite(&snapshot, &original_zone);
                state.queue(effects);

                let borne_cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| {
                        c.get_bearer_id()
                            .ok()
                            .flatten()
                            .filter(|bearer_id| bearer_id == card_id)
                            .map(|_| c.get_id().clone())
                    })
                    .collect();
                for borne_card_id in borne_cards {
                    state.get_card_mut(&borne_card_id).set_bearer_id(None);
                }
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
                    base.modifier_counters.push(counter.clone());
                }
            }
            Effect::SetCardData { card_id, data, .. } => {
                let card = state.get_card_mut(card_id);
                card.set_data(data)?;
            }
            Effect::RangedStrike {
                striker_id,
                target_id,
                ..
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(striker_id);
                let defender = state.get_card(target_id);
                let mut effects = vec![Effect::TakeDamage {
                    card_id: target_id.clone(),
                    from: striker_id.clone(),
                    damage: attacker
                        .get_power(&snapshot)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?,
                    is_strike: true,
                }];
                effects.extend(attacker.after_ranged_attack(state).await?);
                effects.extend(
                    defender
                        .on_defend(state, striker_id)?
                        .into_iter()
                        .map(|e| e.into()),
                );
                state.queue(effects);
            }
            Effect::TeleportCard {
                player_id,
                card_id,
                to_zone,
                ..
            } => {
                let card = state.get_card(&card_id);
                let mut effects = vec![Effect::MoveCard {
                    player_id: player_id.clone(),
                    card_id: card_id.clone(),
                    from: card.get_zone().clone(),
                    to: ZoneQuery::from_zone(to_zone.clone()),
                    tap: false,
                    region: Region::Surface,
                    through_path: None,
                }];

                let carried_cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| {
                        c.get_bearer_id()
                            .ok()
                            .flatten()
                            .filter(|bearer_id| bearer_id == card_id)
                            .map(|_| c.get_id().clone())
                    })
                    .collect();
                let carried_region = state.get_card(&card_id).get_region(state).clone();
                for carried_card_id in carried_cards {
                    let carried = state.get_card_mut(&carried_card_id);
                    carried.set_zone(to_zone.clone());
                    carried.set_region(carried_region.clone());
                }
                let card = state.get_card(&card_id);
                effects.extend(card.on_visit_zone(&state, to_zone).await?);
                state.queue(effects);
            }
            Effect::RearrangeDeck { spells, sites, .. } => {
                let deck = state
                    .decks
                    .get_mut(&state.current_player)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                deck.spells = spells.clone();
                deck.sites = sites.clone();
            }
            Effect::SetCardRegion {
                card_id,
                region,
                tap,
            } => {
                let card = state.get_card(card_id);
                let from_region = card.get_region(&state);
                // Compute change region effects before updating the card's region.
                let mut change_region_effects =
                    card.on_region_change(state, from_region, region)?;

                let borne_artifacts = CardQuery::new().carried_by(card_id).all(state);
                // Append these to the change_region_effects so that the effects of changing
                // region are applied after the region change itself.
                change_region_effects.extend(borne_artifacts.into_iter().map(|artifact_id| {
                    Effect::SetCardRegion {
                        card_id: artifact_id,
                        region: region.clone(),
                        tap: false,
                    }
                }));

                let card = state.get_card_mut(card_id);
                card.set_region(region.clone());
                if *tap {
                    card.set_tapped(true);
                }

                if card.is_minion() {
                    let card = state.get_card(card_id);
                    let snapshot = state.snapshot();
                    let underground_without_burrowing = region == &Region::Underground
                        && !card.has_ability(&snapshot, &Ability::Burrowing);
                    let underwater_without_submerge = region == &Region::Underwater
                        && !card.has_ability(&snapshot, &Ability::Submerge);
                    if underground_without_burrowing || underwater_without_submerge {
                        state.queue_one(Effect::BuryCard {
                            card_id: card_id.clone(),
                        });
                    }
                }

                state.queue(change_region_effects);
            }
            Effect::SetBearer { card_id, bearer_id } => {
                let target = state.get_card_mut(card_id);
                target.set_bearer_id(bearer_id.clone());
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
                if card.get_owner_id() != player_id {
                    return Ok(());
                }

                if card.get_zone() != &Zone::Hand {
                    return Ok(());
                }

                card.set_zone(Zone::Cemetery);
            }
            Effect::SetController { card_id, player_id } => {
                let card = state.get_card_mut(card_id);
                card.get_base_mut().controller_id = player_id.clone();
            }
            Effect::SummonCopy {
                card_name,
                player_id,
                zone,
            } => {
                let mut copy = crate::card::from_name(card_name, player_id);
                copy.get_base_mut().is_token = true;

                let has_charge = copy.has_ability(state, &Ability::Charge);
                let copy_id = copy.get_id().clone();
                state.cards.push(copy);

                let card = state.get_card_mut(&copy_id);
                card.set_zone(zone.clone());
                if !has_charge {
                    card.add_modifier(Ability::SummoningSickness);
                }

                for player in &state.players {
                    crate::game::force_sync(&player.id, state).await?;
                }

                let card = state.get_card(&copy_id);
                let mut effects: Vec<Effect> = vec![];
                effects.extend(card.on_summon(state)?);
                effects.extend(card.genesis(state).await?);
                effects.extend(card.on_visit_zone(state, zone).await?);
                effects.push(Effect::BanishCard {
                    card_id: copy_id,
                    from: zone.clone(),
                });
                state.queue(effects);
            }
        }

        let area_effects: Vec<Effect> = state
            .cards
            .iter()
            .filter_map(|c| c.area_effects(state).ok())
            .flatten()
            .collect();
        state.queue(area_effects);

        self.expire_counters(state).await?;
        self.process_deferred_effects(state, self).await?;
        self.expire_temporary_effects(state, self).await?;

        for player in &state.players {
            crate::game::force_sync(&player.id, state).await?;
        }

        Ok(())
    }
}
