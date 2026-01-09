use crate::{
    card::{Card, FootSoldier, Modifier, Plane, Rubble, UnitBase, Zone},
    game::{BaseAction, Direction, PlayerAction, PlayerId, SoundEffect, Thresholds, pick_card, pick_option},
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, ZoneQuery},
    state::{Phase, State},
};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct ModifierCounter {
    pub id: uuid::Uuid,
    pub modifier: Modifier,
    pub expires_on_effect: Option<EffectQuery>,
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub id: uuid::Uuid,
    pub power: i8,
    pub toughness: i8,
    pub expires_on_effect: Option<EffectQuery>,
}

impl Counter {
    pub fn new(power: i8, toughness: i8, expires_on_effect: Option<EffectQuery>) -> Self {
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
}

#[derive(Debug)]
pub enum Effect {
    SummonToken {
        player_id: uuid::Uuid,
        token_type: TokenType,
        zone: Zone,
    },
    Heal {
        card_id: uuid::Uuid,
        amount: u8,
    },
    ShootProjectile {
        player_id: uuid::Uuid,
        shooter: uuid::Uuid,
        from_zone: Zone,
        direction: Direction,
        damage: u8,
        piercing: bool,
        splash_damage: Option<u8>,
    },
    RemoveModifier {
        card_id: uuid::Uuid,
        modifier: Modifier,
    },
    AddModifierCounter {
        card_id: uuid::Uuid,
        counter: ModifierCounter,
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
    Burrow {
        card_id: uuid::Uuid,
    },
    Submerge {
        card_id: uuid::Uuid,
    },
    MoveCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        from: Zone,
        to: ZoneQuery,
        tap: bool,
        plane: Plane,
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
        zone: Zone,
    },
    SummonCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        zone: Zone,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
    EndTurn {
        player_id: uuid::Uuid,
    },
    StartTurn {
        player_id: uuid::Uuid,
    },
    RemoveResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
    },
    AddResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
    },
    RangedStrike {
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
        damage: u8,
    },
    BanishCard {
        card_id: uuid::Uuid,
        from: Zone,
    },
    BuryCard {
        card_id: uuid::Uuid,
        from: Zone,
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
    DealDamageToTarget {
        player_id: uuid::Uuid,
        query: CardQuery,
        from: uuid::Uuid,
        damage: u8,
    },
    RearrangeDeck {
        spells: Vec<uuid::Uuid>,
        sites: Vec<uuid::Uuid>,
    },
}

fn player_name<'a>(player_id: &uuid::Uuid, state: &'a State) -> &'a str {
    match state.players.iter().find(|p| &p.id == player_id) {
        Some(player) => &player.name,
        None => "Unknown Player",
    }
}

impl Effect {
    pub fn banish_card(card_id: &uuid::Uuid, from: &Zone) -> Self {
        Effect::BanishCard {
            card_id: card_id.clone(),
            from: from.clone(),
        }
    }
    pub fn bury_card(card_id: &uuid::Uuid, from: &Zone) -> Self {
        Effect::BuryCard {
            card_id: card_id.clone(),
            from: from.clone(),
        }
    }

    pub fn tap_card(card_id: &uuid::Uuid) -> Self {
        Effect::TapCard {
            card_id: card_id.clone(),
        }
    }

    pub fn play_card(player_id: &PlayerId, card_id: &uuid::Uuid, zone: &Zone) -> Self {
        Effect::PlayCard {
            player_id: player_id.clone(),
            card_id: card_id.clone(),
            zone: zone.clone(),
        }
    }

    pub fn take_damage(card_id: &uuid::Uuid, from: &uuid::Uuid, damage: u8) -> Self {
        Effect::TakeDamage {
            card_id: card_id.clone(),
            from: from.clone(),
            damage,
        }
    }

    pub fn add_modifier(card_id: &uuid::Uuid, modifier: Modifier, expires_on_effect: Option<EffectQuery>) -> Self {
        Effect::AddModifierCounter {
            card_id: card_id.clone(),
            counter: ModifierCounter {
                id: uuid::Uuid::new_v4(),
                modifier,
                expires_on_effect,
            },
        }
    }

    pub async fn sound_effect(&self) -> anyhow::Result<Option<SoundEffect>> {
        let sound = match self {
            Effect::PlayCard { .. } => Some(SoundEffect::PlayCard),
            _ => None,
        };

        Ok(sound)
    }

    pub async fn description(&self, state: &State) -> anyhow::Result<Option<String>> {
        let desc = match self {
            Effect::SummonToken {
                player_id,
                token_type,
                zone,
            } => {
                let token_name = match token_type {
                    TokenType::Rubble => "Rubble",
                    TokenType::FootSoldier => "Foot Soldier",
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
            Effect::RemoveModifier { .. } => None,
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
            Effect::AddModifierCounter { .. } => None,
            Effect::AddCounter { .. } => None,
            Effect::TeleportCard { .. } => None,
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
                    zone
                ))
            }
            Effect::SummonCard { .. } => None,
            Effect::TapCard { .. } => None,
            Effect::EndTurn { player_id, .. } => Some(format!("{} passes the turn", player_name(player_id, state))),
            Effect::StartTurn { .. } => None,
            Effect::RemoveResources { .. } => None,
            Effect::AddResources { .. } => None,
            Effect::Attack {
                attacker_id,
                defender_id,
                ..
            } => {
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let player = player_name(attacker.get_controller_id(), state);
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
                let player = card.get_controller_id();
                Some(format!("{} buries {}", player_name(player, state), card.get_name()))
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.get_card(card_id);
                let player = card.get_controller_id();
                Some(format!("{} banishes {}", player_name(player, state), card.get_name()))
            }
            Effect::SetCardData { .. } => None,
            Effect::RangedStrike { .. } => None,
            Effect::DealDamageToTarget { .. } => None,
            Effect::TeleportUnitToZone { .. } => None,
            Effect::RearrangeDeck { .. } => None,
            Effect::Burrow { .. } => None,
            Effect::Submerge { .. } => None,
        };

        Ok(desc)
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
        let effects: Vec<Effect> = state
            .cards
            .iter()
            .flat_map(|c| c.replace_effect(state, self))
            .flatten()
            .collect();
        if !effects.is_empty() {
            for effect in effects {
                state.effects.push_back(effect.into());
            }
            return Ok(());
        }

        match self {
            Effect::SummonToken {
                player_id,
                token_type,
                zone,
            } => {
                let mut token: Box<dyn Card> = match token_type {
                    TokenType::Rubble => Box::new(Rubble::new(player_id.clone())),
                    TokenType::FootSoldier => Box::new(FootSoldier::new(player_id.clone())),
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
            Effect::RemoveModifier { card_id, modifier } => {
                let card = state.get_card_mut(card_id);
                card.remove_modifier(modifier);
            }
            Effect::ShootProjectile {
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
                    let units = state
                        .get_units_in_zone(&zone)
                        .iter()
                        .filter(|c| c.can_be_targetted_by(state, player_id))
                        .map(|c| c.get_id().clone())
                        .collect::<Vec<_>>();
                    if units.is_empty() {
                        next_zone = zone.zone_in_direction(direction, 1);
                        continue;
                    }

                    let mut picked_unit_id = units[0];
                    if units.len() >= 1 {
                        let prompt = "Pick a unit to shoot";
                        picked_unit_id = pick_card(player_id, &units, state, prompt).await?;
                    }

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
                    next_zone = zone.zone_in_direction(direction, 1);
                }

                for effect in effects {
                    state.effects.push_back(effect.into());
                }
            }
            Effect::TeleportCard { card_id, to, .. } => {
                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                card.set_zone(to.clone());
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
                            let card = state.get_card_mut(card_id);
                            card.set_zone(zone.clone());
                            if *tap {
                                card.get_base_mut().tapped = true;
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
                        let card = state.get_card_mut(card_id);
                        card.set_zone(zone.clone());
                        if *tap {
                            card.get_base_mut().tapped = true;
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
                    let picked_option_idx = pick_option(player_id, &option_labels, state, "Draw a card").await?;
                    state.effects.extend(
                        options[picked_option_idx]
                            .on_select(player_id, state)
                            .await?
                            .into_iter()
                            .map(|e| e.into()),
                    );
                }
            }
            Effect::PlayMagic { card_id, caster_id, .. } => {
                let card = state
                    .cards
                    .iter()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                let mana_cost = card.get_mana_cost(&state);
                state.effects.push_back(
                    Effect::RemoveResources {
                        player_id: card.get_owner_id().clone(),
                        mana: mana_cost,
                        thresholds: Thresholds::new(),
                    }
                    .into(),
                );
                state.effects.push_back(
                    Effect::MoveCard {
                        player_id: card.get_controller_id().clone(),
                        card_id: card.get_id().clone(),
                        from: card.get_zone().clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: Zone::Cemetery,
                        },
                        tap: false,
                        plane: Plane::None,
                        through_path: None,
                    }
                    .into(),
                );

                let snapshot = state.snapshot();
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                let effects = card.on_cast(&snapshot, caster_id).await?.into_iter().map(|e| e.into());
                state.effects.extend(effects);
            }
            Effect::PlayCard { card_id, zone, .. } => {
                let snapshot = state.snapshot();
                let card = state
                    .cards
                    .iter_mut()
                    .find(|c| c.get_id() == card_id)
                    .expect("to find card");
                let cast_effects = card.on_summon(&snapshot)?;
                card.set_zone(zone.clone());
                if !card.has_modifier(&snapshot, &Modifier::Charge) {
                    card.add_modifier(Modifier::SummoningSickness);
                }

                let mut effects = card.genesis(&snapshot).await?;
                effects.extend(card.on_visit_zone(&snapshot, zone).await?);
                let mana_cost = card.get_mana_cost(&snapshot);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                });
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
                if !card.has_modifier(&snapshot, &Modifier::Charge) {
                    card.add_modifier(Modifier::SummoningSickness);
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
            Effect::StartTurn { player_id, .. } => {
                let previous_player = state.current_player.clone();
                state
                    .get_sender()
                    .send(ServerMessage::Wait {
                        player_id: previous_player.clone(),
                        prompt: "Waiting for other player".to_string(),
                    })
                    .await?;
                let cards = state
                    .cards
                    .iter_mut()
                    .filter(|c| c.get_owner_id() == &state.current_player);
                for card in cards {
                    card.get_base_mut().tapped = false;
                    card.remove_modifier(&Modifier::SummoningSickness);
                }

                for card in state.cards.iter().filter(|c| c.get_owner_id() == &state.current_player) {
                    let effects = card.on_turn_start(state).await;
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                }

                let available_mana: u8 = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| c.get_zone().is_in_play())
                    .filter_map(|c| match c.get_site_base() {
                        Some(site_base) => Some(site_base.provided_mana),
                        None => None,
                    })
                    .sum();
                let player_resources = state.get_player_resources_mut(player_id)?;
                player_resources.mana = available_mana;

                let options: Vec<BaseAction> = vec![BaseAction::DrawSite, BaseAction::DrawSpell];
                let option_labels: Vec<String> = options.iter().map(|a| a.get_name().to_string()).collect();
                let prompt = "Start Turn: Pick card to draw";
                let picked_option_idx = pick_option(player_id, &option_labels, state, prompt).await?;
                let effects = options[picked_option_idx].on_select(player_id, state).await?;
                state.effects.extend(effects.into_iter().map(|e| e.into()));

                state.current_player = player_id.clone();
                state.turns += 1;
                state
                    .get_sender()
                    .send(ServerMessage::Resume {
                        player_id: previous_player,
                    })
                    .await?;
            }
            Effect::RemoveResources {
                player_id,
                mana,
                thresholds,
                ..
            } => {
                let player_resources = state.get_player_resources_mut(player_id)?;
                player_resources.mana -= mana;
                player_resources.thresholds.air = player_resources.thresholds.air.saturating_sub(thresholds.air);
                player_resources.thresholds.water = player_resources.thresholds.water.saturating_sub(thresholds.water);
                player_resources.thresholds.fire = player_resources.thresholds.fire.saturating_sub(thresholds.fire);
                player_resources.thresholds.earth = player_resources.thresholds.earth.saturating_sub(thresholds.earth);
            }
            Effect::EndTurn { player_id, .. } => {
                for card in state.cards.iter().filter(|c| c.get_zone().is_in_play()) {
                    let effects = card.on_turn_end(state).await?;
                    state.effects.extend(effects.into_iter().map(|e| e.into()));
                }

                let resources = state
                    .resources
                    .get_mut(player_id)
                    .ok_or(anyhow::anyhow!("Player resources not found"))?;
                resources.mana = 0;
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
            Effect::AddResources {
                player_id,
                mana,
                thresholds,
                ..
            } => {
                let player_resources = state.get_player_resources_mut(player_id)?;
                player_resources.mana += mana;
                player_resources.thresholds.air += thresholds.air;
                player_resources.thresholds.water += thresholds.water;
                player_resources.thresholds.fire += thresholds.fire;
                player_resources.thresholds.earth += thresholds.earth;
            }
            Effect::Attack {
                attacker_id,
                defender_id,
                ..
            } => {
                let snapshot = state.snapshot();
                let attacker = state.get_card(attacker_id);
                let defender = state.get_card(defender_id);
                let mut effects = vec![
                    Effect::MoveCard {
                        player_id: attacker.get_controller_id().clone(),
                        card_id: attacker_id.clone(),
                        from: attacker.get_zone().clone(),
                        to: ZoneQuery::Specific {
                            id: uuid::Uuid::new_v4(),
                            zone: defender.get_zone().clone(),
                        },
                        tap: true,
                        plane: attacker.get_base().plane.clone(),
                        through_path: None,
                    },
                    Effect::TakeDamage {
                        card_id: defender_id.clone(),
                        from: attacker_id.clone(),
                        damage: attacker
                            .get_power(&snapshot)?
                            .ok_or(anyhow::anyhow!("attacker has no power"))?,
                    },
                ];
                effects.extend(attacker.after_attack(state).await?);
                effects.extend(defender.on_defend(state, attacker_id)?.into_iter().map(|e| e.into()));
                state.effects.extend(effects.into_iter().map(|e| e.into()));
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
                card.set_zone(Zone::Banish);
            }
            Effect::BuryCard { card_id, from, .. } => {
                {
                    let card = state.get_card_mut(card_id);
                    card.set_zone(Zone::Cemetery);
                }

                let snapshot = state.snapshot();
                let card = state.get_card_mut(card_id);
                let effects = card.deathrite(&snapshot, from);
                state.effects.extend(effects.into_iter().map(|e| e.into()));
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
            Effect::AddModifierCounter { card_id, counter, .. } => {
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
                effects.extend(attacker.after_attack(state).await?);
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
                        plane: Plane::Surface,
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
            Effect::Burrow { card_id, .. } => {
                let card = state.get_card_mut(card_id);
                card.get_base_mut().plane = Plane::Underground;
            }
            Effect::Submerge { card_id, .. } => {
                let card = state.get_card_mut(card_id);
                card.get_base_mut().plane = Plane::Submerged;
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

        Ok(())
    }
}
