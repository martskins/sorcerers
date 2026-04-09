use crate::{
    card::{Ability, Card, CardType, FootSoldier, Frog, Region, Rubble, UnitBase, Zone},
    game::{pick_card, pick_option, BaseAction, Direction, PlayerAction, PlayerId, SoundEffect},
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, QueryCache, ZoneQuery},
    state::{CardMatcher, DeferredEffect, Phase, State, TemporaryEffect},
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
        player_id: uuid::Uuid,
    },
    SummonToken {
        player_id: uuid::Uuid,
        token_type: TokenType,
        zone: Zone,
    },
    Heal {
        card_id: uuid::Uuid,
        amount: u16,
    },
    ShootProjectile {
        id: uuid::Uuid,
        player_id: uuid::Uuid,
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
    TeleportCard {
        card_id: uuid::Uuid,
        from: Zone,
        to: Zone,
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
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
    },
    MoveCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        from: Zone,
        to: ZoneQuery,
        tap: bool,
        region: Region,
        through_path: Option<Vec<Zone>>,
    },
    DrawSite {
        player_id: uuid::Uuid,
        count: u8,
    },
    DrawSpell {
        player_id: uuid::Uuid,
        count: u8,
    },
    DrawCard {
        player_id: uuid::Uuid,
        count: u8,
    },
    PlayMagic {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        caster_id: uuid::Uuid,
        from: Zone,
    },
    PlayCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        zone: ZoneQuery,
    },
    SummonCard {
        player_id: uuid::Uuid,
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
        player_id: uuid::Uuid,
    },
    StartTurn {
        player_id: uuid::Uuid,
    },
    ConsumeMana {
        player_id: uuid::Uuid,
        mana: u8,
    },
    AddMana {
        player_id: uuid::Uuid,
        mana: u8,
    },
    RangedStrike {
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
    },
    Strike {
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
    },
    Attack {
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
    },
    TakeDamage {
        card_id: uuid::Uuid,
        from: uuid::Uuid,
        damage: u16,
    },
    BanishCard {
        card_id: uuid::Uuid,
        from: Zone,
    },
    BuryCard {
        card_id: uuid::Uuid,
    },
    SetCardData {
        card_id: uuid::Uuid,
        data: Box<dyn std::any::Any + Send + Sync>,
    },
    TeleportUnitToZone {
        player_id: PlayerId,
        unit_query: CardQuery,
        zone_query: ZoneQuery,
    },
    DealDamageAllUnitsInZone {
        player_id: uuid::Uuid,
        zone: ZoneQuery,
        from: uuid::Uuid,
        damage: u16,
    },
    DealDamageToTarget {
        player_id: uuid::Uuid,
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
        player_id: uuid::Uuid,
    },
}

fn player_name<'a>(player_id: &uuid::Uuid, state: &'a State) -> &'a str {
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
            Effect::TeleportCard { card_id, .. } => Some(card_id),
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
            Effect::TapCard { card_id } => Some(card_id),
            Effect::UntapCard { card_id } => Some(card_id),
            Effect::EndTurn { player_id } => Some(player_id),
            Effect::StartTurn { player_id } => Some(player_id),
            Effect::ConsumeMana { player_id, .. } => Some(player_id),
            Effect::AddMana { player_id, .. } => Some(player_id),
            Effect::Strike { attacker_id, .. } => Some(attacker_id),
            Effect::RangedStrike { attacker_id, .. } => Some(attacker_id),
            Effect::Attack { attacker_id, .. } => Some(attacker_id),
            Effect::TakeDamage { card_id, .. } => Some(card_id),
            Effect::BanishCard { card_id, .. } => Some(card_id),
            Effect::BuryCard { card_id } => Some(card_id),
            Effect::SetCardData { card_id, .. } => Some(card_id),
            Effect::TeleportUnitToZone { player_id, .. } => Some(player_id),
            Effect::DealDamageAllUnitsInZone { from, .. } => Some(from),
            Effect::DealDamageToTarget { from, .. } => Some(from),
            Effect::RearrangeDeck { .. } => None,
            Effect::AddDeferredEffect { .. } => None,
            Effect::AddTemporaryEffect { .. } => None,
            Effect::SetBearer { card_id, .. } => Some(card_id),
            Effect::ShuffleDeck { .. } => None,
        }
    }

    pub async fn description(&self, state: &State) -> anyhow::Result<Option<String>> {
        let desc = match self {
            Effect::PlayerLost { player_id } => Some(format!("{} has lost the game", player_name(player_id, state))),
            Effect::SetCardRegion { card_id, region, .. } => {
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
            Effect::AddCounter { .. } => None,
            Effect::TeleportCard { .. } => None,
            Effect::SetCardZone { .. } => None,
            Effect::DiscardCard { card_id, player_id } => {
                let card = state.get_card(card_id).get_name();
                Some(format!("{} discards {}", player_name(player_id, state), card))
            }
            Effect::MoveCard {
                player_id,
                to,
                through_path,
                card_id,
                from,
                ..
            } => {
                // Calling resolve here should result in us getting the result from the cache,
                // as the effect should have been applied already.
                let card = state.get_card(card_id);
                if card.get_zone() != from {
                    return Ok(None);
                }

                let card_name = card.get_name();
                match through_path {
                    Some(path) => Some(format!(
                        "{} moves {} to {} through path {}",
                        player_name(&player_id, state),
                        card_name,
                        to.resolve(player_id, state).await?,
                        path.iter().map(|c| format!("{}", c)).collect::<Vec<_>>().join(" -> "),
                    )),
                    None => Some(format!(
                        "{} moves {} to {}",
                        player_name(&player_id, state),
                        card_name,
                        to.resolve(player_id, state).await?,
                    )),
                }
            }
            Effect::DrawCard { .. } => None,
            Effect::DrawSite { player_id, count, .. } => {
                let sites = if *count == 1 { "site" } else { "sites" };
                Some(format!("{} draws {} {}", player_name(player_id, state), count, sites))
            }
            Effect::DrawSpell { player_id, count, .. } => {
                let spells = if *count == 1 { "site" } else { "sites" };
                Some(format!("{} draws {} {}", player_name(player_id, state), count, spells))
            }
            Effect::PlayMagic { .. } => None,
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
            Effect::SummonCard { .. } => None,
            Effect::TapCard { .. } => None,
            Effect::UntapCard { .. } => None,
            Effect::EndTurn { player_id, .. } => Some(format!("{} passes the turn", player_name(player_id, state))),
            Effect::StartTurn { .. } => None,
            Effect::ConsumeMana { .. } => None,
            Effect::AddMana { .. } => None,
            Effect::Strike {
                attacker_id,
                defender_id,
            } => Some(format!(
                "{} strikes {} with {}",
                player_name(&state.get_card(attacker_id).get_controller_id(state), state),
                state.get_card(defender_id).get_name(),
                state.get_card(attacker_id).get_name(),
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
                card_id, from, damage, ..
            } => {
                let attacker = state.get_card(from).get_name();
                let defender = state.get_card(card_id).get_name();
                Some(format!("{} takes {} damage from {}", defender, damage, attacker))
            }
            Effect::BuryCard { card_id, .. } => {
                let card = state.get_card(card_id);
                let player = card.get_controller_id(state);
                Some(format!("{} buries {}", player_name(&player, state), card.get_name()))
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.get_card(card_id);
                let player = card.get_controller_id(state);
                Some(format!("{} banishes {}", player_name(&player, state), card.get_name()))
            }
            Effect::SetCardData { .. } => None,
            Effect::RangedStrike { .. } => None,
            Effect::DealDamageToTarget { .. } => None,
            Effect::DealDamageAllUnitsInZone { .. } => None,
            Effect::TeleportUnitToZone { .. } => None,
            Effect::RearrangeDeck { .. } => None,
            Effect::SetBearer { card_id, bearer_id } => {
                let card = state.get_card(card_id);
                match bearer_id {
                    Some(carrier_id) => {
                        let carrier = state.get_card(carrier_id);
                        Some(format!("{} is now carried by {}", card.get_name(), carrier.get_name()))
                    }
                    None => Some(format!("{} is no longer carried", card.get_name())),
                }
            }
            Effect::ShuffleDeck { player_id } => Some(format!("{} shuffles their deck", player_name(player_id, state))),
        };

        Ok(desc)
    }

    async fn process_deferred_effects(&self, state: &mut State, effect: &Effect) -> anyhow::Result<()> {
        let mut effects_to_remove = vec![];
        for (idx, de) in state.deferred_effects.iter().enumerate() {
            if de.trigger_on_effect.matches(effect, state).await? {
                if let Some(source_id) = effect.source_id() {
                    let effects = (de.on_effect)(state, source_id, effect).await?;
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                }

                effects_to_remove.push(idx);
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

    async fn expire_temporary_effects(&self, state: &mut State, effect: &Effect) -> anyhow::Result<()> {
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
            for counter in &card.get_unit_base().unwrap_or(&UnitBase::default()).modifier_counters {
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
            .filter(|c| c.get_unit_base().unwrap_or(&UnitBase::default()).power_counters.len() > 0)
            .collect();
        let mut card_counters_to_remove: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = vec![];
        for card in cards_with_counters {
            let mut to_remove: Vec<uuid::Uuid> = vec![];
            for counter in &card.get_unit_base().unwrap_or(&UnitBase::default()).power_counters {
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
            for effect in effects {
                state.effects.push_back(effect.into());
            }

            return Ok(());
        }

        match self {
            Effect::PlayerLost { player_id } => {
                state.loosers.insert(player_id.clone());
            }
            Effect::AddDeferredEffect { effect } => {
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
                                    let picked_unit_id = pick_card(player_id, &units, state, prompt).await?;
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
                        effects.push(Effect::take_damage(&picked_unit_id, shooter, *damage));
                        if let Some(splash_damage) = splash_damage {
                            let splash_effects = state
                                .get_units_in_zone(&zone)
                                .iter()
                                .filter(|c| c.get_id() != &picked_unit_id)
                                .map(|c| Effect::take_damage(c.get_id(), shooter, *splash_damage))
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
            Effect::TeleportCard { card_id, to, .. } => {
                let snapshot = state.snapshot();
                let bearer_id = {
                    let card = state.get_card_mut(card_id);
                    card.set_zone(to.clone());
                    card.get_bearer_id()?
                };
                if let Some(bearer_id) = bearer_id {
                    let bearer = state.get_card(&bearer_id);
                    let card = state.get_card(card_id);
                    if bearer.get_zone() != card.get_zone() || bearer.get_region(state) != card.get_region(state) {
                        state.get_card_mut(card_id).set_bearer_id(None);
                    }
                }
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
                let carried_region = state.get_card(card_id).get_region(state).clone();
                for carried_card_id in carried_cards {
                    let carried = state.get_card_mut(&carried_card_id);
                    carried.set_zone(to.clone());
                    carried.get_base_mut().region = carried_region.clone();
                }
                let card = state.get_card(card_id);
                let effects = card.on_visit_zone(&snapshot, to).await?;
                state.effects.extend(effects.into_iter().map(|e| e.into()));
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
                            let zone = ZoneQuery::Specific {
                                id: uuid::Uuid::new_v4(),
                                zone: zone.clone(),
                            }
                            .resolve(player_id, state)
                            .await?;
                            let bearer_id = {
                                let card = state.get_card_mut(card_id);
                                card.set_zone(zone.clone());
                                if *tap {
                                    card.get_base_mut().tapped = true;
                                }
                                card.get_bearer_id()?
                            };
                            if let Some(bearer_id) = bearer_id {
                                let bearer = state.get_card(&bearer_id);
                                let card = state.get_card(card_id);
                                if bearer.get_zone() != card.get_zone()
                                    || bearer.get_region(state) != card.get_region(state)
                                {
                                    state.get_card_mut(card_id).set_bearer_id(None);
                                }
                            }

                            let card = state.get_card(card_id);
                            let mut effects = card.on_move(&snapshot, path).await?;
                            effects.extend(card.on_visit_zone(&snapshot, &zone).await?);
                            if let Some(site) = zone.get_site(state) {
                                effects.extend(site.on_card_enter(state, card_id));
                            }

                            state.effects.extend(effects.into_iter().map(|e| e.into()));
                        }
                    }
                    None => {
                        let snapshot = state.snapshot();
                        let zone = to.resolve(player_id, state).await?;
                        let bearer_id = {
                            let card = state.get_card_mut(card_id);
                            card.set_zone(zone.clone());
                            if *tap {
                                card.get_base_mut().tapped = true;
                            }
                            card.get_bearer_id()?
                        };
                        if let Some(bearer_id) = bearer_id {
                            let bearer = state.get_card(&bearer_id);
                            let card = state.get_card(card_id);
                            if bearer.get_zone() != card.get_zone()
                                || bearer.get_region(state) != card.get_region(state)
                            {
                                state.get_card_mut(card_id).set_bearer_id(None);
                            }
                        }

                        let card = state.get_card(card_id);
                        let path = vec![from.clone(), zone.clone()];
                        let mut effects = card.on_move(&snapshot, &path).await?;
                        effects.extend(card.on_visit_zone(&snapshot, &zone).await?);
                        if let Some(site) = zone.get_site(state) {
                            effects.extend(site.on_card_enter(state, card_id));
                        }

                        state.effects.extend(effects.into_iter().map(|e| e.into()));
                    }
                }
            }
            Effect::DrawSite { player_id, count, .. } => {
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
            Effect::DrawSpell { player_id, count, .. } => {
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
            Effect::DrawCard { player_id, count, .. } => {
                for _ in 0..*count {
                    let options: Vec<BaseAction> = vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                    let option_labels = options.iter().map(|a| a.get_name().to_string()).collect::<Vec<_>>();
                    let picked_option_idx = pick_option(player_id, &option_labels, state, "Draw a card", false).await?;
                    state.effects.extend(
                        options[picked_option_idx]
                            .on_select(player_id, state)
                            .await?
                            .into_iter()
                            .map(|e| e.into()),
                    );
                }
            }
            Effect::PlayMagic {
                card_id,
                player_id,
                caster_id,
                ..
            } => {
                let costs = state.get_card(card_id).get_costs(&state)?.clone();
                let paid_cost = costs.pay(state, player_id).await?;

                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card
                    .on_cast(&snapshot, caster_id, paid_cost)
                    .await?
                    .into_iter()
                    .map(|e| e.into());

                // Set zone after on_cast so that the card is not in the cemetery during casting.
                card.set_zone(Zone::Cemetery);
                state.effects.extend(effects);
            }
            Effect::PlayCard {
                card_id,
                player_id,
                zone,
                ..
            } => {
                let zone = zone.resolve(player_id, state).await?;
                let snapshot = state.snapshot();
                let costs = state.get_card(card_id).get_costs(&snapshot)?.clone();
                Box::pin(costs.pay(state, &player_id)).await?;
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");

                // If playing a site and there is a rubble on that zone, remove it.
                if card.is_site() {
                    if let Some(site) = zone.get_site(&snapshot) {
                        if site.get_name() == Rubble::NAME {
                            state.effects.push_back(
                                Effect::BanishCard {
                                    card_id: site.get_id().clone(),
                                    from: zone.clone(),
                                }
                                .into(),
                            );
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
                state.effects.extend(effects.into_iter().map(|e| e.into()));
                state.effects.extend(cast_effects.into_iter().map(|e| e.into()));
            }
            Effect::SummonCard { card_id, zone, .. } => {
                let snapshot = state.snapshot();
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                let cast_effects = card.on_summon(&snapshot)?;
                card.set_zone(zone.clone());
                if !card.has_ability(&snapshot, &Ability::Charge) {
                    card.add_modifier(Ability::SummoningSickness);
                }

                let mut effects = card.genesis(&snapshot).await?;
                effects.extend(card.on_visit_zone(&snapshot, zone).await?);
                state.effects.extend(effects.into_iter().map(|e| e.into()));
                state.effects.extend(cast_effects.into_iter().map(|e| e.into()));
            }
            Effect::TapCard { card_id, .. } => {
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                card.get_base_mut().tapped = true;
            }
            Effect::UntapCard { card_id, .. } => {
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                card.get_base_mut().tapped = false;
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
                    card.get_base_mut().tapped = false;
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

                for card in state.cards.iter().filter(|c| c.get_owner_id() == &state.current_player) {
                    if !card.get_zone().is_in_play() {
                        continue;
                    }

                    let effects = card.on_turn_start(state).await?;
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                }

                let options: Vec<BaseAction> = vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                let option_labels: Vec<String> = options.iter().map(|a| a.get_name().to_string()).collect();
                let prompt = "Start Turn: Pick card to draw";
                let picked_option_idx = pick_option(player_id, &option_labels, state, prompt, false).await?;
                let effects = options[picked_option_idx].on_select(player_id, state).await?;
                state.effects.extend(effects.into_iter().map(|e| e.into()));

                state.turns += 1;
                state
                    .get_sender()
                    .send(ServerMessage::Resume {
                        player_id: previous_player,
                    })
                    .await?;
            }
            Effect::ConsumeMana { player_id, mana, .. } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana = player_mana.saturating_sub(*mana);
            }
            Effect::EndTurn { player_id, .. } => {
                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let effects = card.on_turn_end(state).await?;
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                }

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
                state.effects.push_front(
                    Effect::StartTurn {
                        player_id: next_player.id.clone(),
                    }
                    .into(),
                );
            }
            Effect::AddMana { player_id, mana, .. } => {
                let player_mana = state.get_player_mana_mut(player_id);
                *player_mana += mana;
            }
            Effect::Strike {
                attacker_id,
                defender_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let mut effects = vec![Effect::TakeDamage {
                    card_id: defender_id.clone(),
                    from: attacker_id.clone(),
                    damage: attacker
                        .get_power(&snapshot)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?,
                }];

                effects.extend(defender.on_defend(state, attacker_id)?.into_iter().map(|e| e.into()));
                effects.reverse();
                state.effects.extend(effects.into_iter().map(|e| e.into()));
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
                    to: ZoneQuery::Specific {
                        id: uuid::Uuid::new_v4(),
                        zone: defender.get_zone().clone(),
                    },
                    tap: true,
                    region: attacker.get_base().region.clone(),
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
                effects.push(Effect::TakeDamage {
                    card_id: first_defender_id.clone(),
                    from: first_striker_id.clone(),
                    damage: first_striker
                        .get_power(&snapshot)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?,
                });

                let mut snapshot = state.snapshot();
                Box::pin(snapshot.apply_effects_without_log()).await?;

                let killed_defender = state.get_card(first_defender_id).get_zone() == &Zone::Cemetery;
                if killed_defender
                    && first_striker.has_ability(state, &Ability::FirstStrike)
                    && !first_defender.has_ability(state, &Ability::FirstStrike)
                {
                    // If the first striker killed the defender before it could strike back, skip the defender's strike.
                    effects.reverse();
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                    return Ok(());
                }

                effects.extend(defender.on_defend(state, attacker_id)?.into_iter().map(|e| e.into()));
                effects.reverse();
                state.effects.extend(effects.into_iter().map(|e| e.into()));
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
                    state.effects.push_back(
                        Effect::TakeDamage {
                            card_id: unit_id,
                            from: from.clone(),
                            damage: *damage,
                        }
                        .into(),
                    );
                }
            }
            Effect::DealDamageToTarget {
                player_id,
                query,
                from,
                damage,
                ..
            } => {
                let target = query.resolve(player_id, state).await?;
                state.effects.push_back(
                    Effect::TakeDamage {
                        card_id: target,
                        from: from.clone(),
                        damage: *damage,
                    }
                    .into(),
                );
            }
            Effect::TakeDamage {
                card_id, damage, from, ..
            } => {
                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card.on_take_damage(&snapshot, from, *damage)?;
                for effect in effects {
                    state.effects.push_back(effect.into());
                }
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
            Effect::BuryCard { card_id, .. } => {
                let card = state.get_card_mut(card_id);
                let original_zone = card.get_zone().clone();
                card.set_bearer_id(None);
                card.set_zone(Zone::Cemetery);

                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card.deathrite(&snapshot, &original_zone);
                state.effects.extend(effects.into_iter().map(|e| e.into()));

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
            Effect::AddCounter { card_id, counter, .. } => {
                let card = state.get_card_mut(card_id);
                if card.is_unit() {
                    let base = card
                        .get_unit_base_mut()
                        .ok_or(anyhow::anyhow!("card has no unit base"))?;
                    base.power_counters.push(counter.clone());
                }
            }
            Effect::AddAbilityCounter { card_id, counter, .. } => {
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
                attacker_id,
                defender_id,
                ..
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let mut effects = vec![Effect::TakeDamage {
                    card_id: defender_id.clone(),
                    from: attacker_id.clone(),
                    damage: attacker
                        .get_power(&snapshot)?
                        .ok_or(anyhow::anyhow!("attacker has no power"))?,
                }];
                effects.extend(attacker.after_ranged_attack(state).await?);
                effects.extend(defender.on_defend(state, attacker_id)?.into_iter().map(|e| e.into()));
                state.effects.extend(effects.into_iter().map(|e| e.into()));
            }
            Effect::TeleportUnitToZone {
                player_id,
                unit_query,
                zone_query,
                ..
            } => {
                let unit_id = unit_query.resolve(player_id, state).await?;
                let unit = state.get_card(&unit_id);
                let zone = zone_query.resolve(player_id, state).await?;
                state.effects.push_back(
                    Effect::MoveCard {
                        player_id: player_id.clone(),
                        card_id: unit_id.clone(),
                        from: unit.get_zone().clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone,
                        },
                        tap: false,
                        region: Region::Surface,
                        through_path: None,
                    }
                    .into(),
                );
            }
            Effect::RearrangeDeck { spells, sites, .. } => {
                let deck = state
                    .decks
                    .get_mut(&state.current_player)
                    .ok_or(anyhow::anyhow!("failed to find player deck"))?;
                deck.spells = spells.clone();
                deck.sites = sites.clone();
            }
            Effect::SetCardRegion { card_id, region, tap } => {
                let card = state.get_card(card_id);
                let from_region = card.get_region(&state);
                // Compute change region effects before updating the card's region.
                let mut change_region_effects = card.on_region_change(state, from_region, region)?;

                if card.is_unit() {
                    let borne_artifacts = CardMatcher::new()
                        .with_card_type(CardType::Artifact)
                        .iter(state)
                        .filter_map(|c| c.get_artifact())
                        .filter(|a| a.get_bearer().unwrap_or_default() == Some(card_id.clone()))
                        .map(|c| c.get_id().clone())
                        .collect::<Vec<_>>();
                    // Append these to the change_region_effects so that the effects of changing
                    // region are applied after the region change itself.
                    change_region_effects.extend(borne_artifacts.into_iter().map(|artifact_id| {
                        Effect::SetCardRegion {
                            card_id: artifact_id,
                            region: region.clone(),
                            tap: false,
                        }
                    }));
                }

                let card = state.get_card_mut(card_id);
                card.get_base_mut().region = region.clone();
                if *tap {
                    card.get_base_mut().tapped = true;
                }

                if card.is_minion() {
                    let card = state.get_card(card_id);
                    let snapshot = state.snapshot();
                    let underground_without_burrowing =
                        region == &Region::Underground && !card.has_ability(&snapshot, &Ability::Burrowing);
                    let underwater_without_submerge =
                        region == &Region::Underwater && !card.has_ability(&snapshot, &Ability::Submerge);
                    if underground_without_burrowing || underwater_without_submerge {
                        state.effects.push_back(
                            Effect::BuryCard {
                                card_id: card_id.clone(),
                            }
                            .into(),
                        );
                    }
                }

                state
                    .effects
                    .extend(change_region_effects.into_iter().map(|e| e.into()));
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
        }

        let area_effects: Vec<Effect> = state
            .cards
            .iter()
            .filter_map(|c| c.area_effects(state).ok())
            .flatten()
            .collect();
        for effect in area_effects {
            state.effects.push_back(effect.into());
        }

        self.expire_counters(state).await?;
        self.process_deferred_effects(state, self).await?;
        self.expire_temporary_effects(state, self).await?;

        Ok(())
    }
}
