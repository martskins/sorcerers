use crate::{
    card::{Card, Modifier, Plane, SiteBase, Zone},
    game::{Action, BaseAction, Direction, PlayerAction, PlayerId, Thresholds, pick_action, pick_card},
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
pub enum Effect {
    ShootProjectile {
        player_id: uuid::Uuid,
        shooter: uuid::Uuid,
        from_zone: Zone,
        direction: Direction,
        damage: u8,
        piercing: bool,
        splash_damage: Option<u8>,
    },
    AddModifier {
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
    AddCard {
        card: Box<dyn crate::card::Card>,
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
    PreEndTurn {
        player_id: uuid::Uuid,
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
        health: u8,
    },
    AddResources {
        player_id: uuid::Uuid,
        mana: u8,
        thresholds: Thresholds,
        health: u8,
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

fn player_name(player_id: &uuid::Uuid, state: &State) -> &'static str {
    if player_id == &state.player_one {
        "Player 1"
    } else {
        "Player 2"
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
        Effect::AddModifier {
            card_id: card_id.clone(),
            counter: ModifierCounter {
                id: uuid::Uuid::new_v4(),
                modifier,
                expires_on_effect,
            },
        }
    }

    pub fn description(&self, state: &State) -> Option<String> {
        match self {
            Effect::ShootProjectile {
                player_id,
                shooter,
                damage,
                direction,
                ..
            } => {
                let shooter_card = state.get_card(shooter).unwrap().get_name();
                Some(format!(
                    "{} shoots a projectile for {} damage from {} in direction {}",
                    player_name(player_id, state),
                    damage,
                    shooter_card,
                    direction.get_name()
                ))
            }
            Effect::AddCard { .. } => None,
            Effect::AddModifier { .. } => None,
            Effect::AddCounter { .. } => None,
            Effect::TeleportCard { .. } => None,
            Effect::MoveCard { .. } => None,
            Effect::DrawCard { .. } => None,
            Effect::DrawSite { player_id, count } => {
                let sites = if *count == 1 { "site" } else { "sites" };
                Some(format!("{} draws {} {}", player_name(player_id, state), count, sites))
            }
            Effect::DrawSpell { player_id, count } => {
                let spells = if *count == 1 { "site" } else { "sites" };
                Some(format!("{} draws {} {}", player_name(player_id, state), count, spells))
            }
            Effect::PlayMagic { .. } => None,
            Effect::PlayCard {
                player_id,
                card_id,
                zone,
            } => {
                let card = state.get_card(card_id).unwrap().get_name();
                Some(format!(
                    "{} plays {} in zone {:?}",
                    player_name(player_id, state),
                    card,
                    zone
                ))
            }
            Effect::SummonCard { .. } => None,
            Effect::TapCard { .. } => None,
            Effect::PreEndTurn { .. } => None,
            Effect::EndTurn { .. } => None,
            Effect::StartTurn { .. } => None,
            Effect::RemoveResources { .. } => None,
            Effect::AddResources { .. } => None,
            Effect::Attack {
                attacker_id,
                defender_id,
            } => {
                let attacker = state.get_card(attacker_id).unwrap();
                let defender = state.get_card(defender_id).unwrap();
                let player = player_name(attacker.get_controller_id(), state);
                Some(format!(
                    "{} attacks {} with {}",
                    player,
                    defender.get_name(),
                    attacker.get_name()
                ))
            }
            Effect::TakeDamage { card_id, from, damage } => {
                let attacker = state.get_card(from).unwrap().get_name();
                let defender = state.get_card(card_id).unwrap().get_name();
                Some(format!("{} takes {} damage from {}", defender, damage, attacker))
            }
            Effect::BuryCard { card_id, .. } => {
                let card = state.get_card(card_id).unwrap();
                let player = card.get_controller_id();
                Some(format!("{} buries {}", player_name(player, state), card.get_name()))
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.get_card(card_id).unwrap();
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
        }
    }

    async fn expire_counters(&self, state: &mut State) {
        let modified_cards: Vec<&Box<dyn Card>> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_unit_base().unwrap().modifier_counters.len() > 0)
            .collect();
        let mut card_modifiers_to_remove: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = vec![];
        for card in modified_cards {
            let mut to_remove: Vec<uuid::Uuid> = vec![];
            for counter in &card.get_unit_base().unwrap().modifier_counters {
                if let Some(effect_query) = &counter.expires_on_effect {
                    if effect_query.matches(self, state).await {
                        to_remove.push(counter.id);
                    }
                }
            }

            if !to_remove.is_empty() {
                card_modifiers_to_remove.push((card.get_id().clone(), to_remove));
            }
        }

        for (card_id, to_remove) in card_modifiers_to_remove {
            let card_mut = state.get_card_mut(&card_id).unwrap();
            for counter_id in to_remove {
                card_mut.remove_modifier_counter(&counter_id);
            }
        }

        let cards_with_counters: Vec<&Box<dyn Card>> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_unit_base().unwrap().power_counters.len() > 0)
            .collect();
        let mut card_counters_to_remove: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = vec![];
        for card in cards_with_counters {
            let mut to_remove: Vec<uuid::Uuid> = vec![];
            for counter in &card.get_unit_base().unwrap().power_counters {
                if let Some(effect_query) = &counter.expires_on_effect {
                    if effect_query.matches(self, state).await {
                        to_remove.push(counter.id);
                    }
                }
            }

            if !to_remove.is_empty() {
                card_counters_to_remove.push((card.get_id().clone(), to_remove));
            }
        }

        for (card_id, to_remove) in card_counters_to_remove {
            let card_mut = state.get_card_mut(&card_id).unwrap();
            for counter_id in to_remove {
                card_mut.remove_counter(&counter_id);
            }
        }
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
                state.effects.push_front(effect);
            }
            return Ok(());
        }
        match self {
            Effect::ShootProjectile {
                player_id,
                shooter,
                from_zone,
                direction,
                damage,
                piercing,
                splash_damage,
            } => {
                let mut effects = vec![];
                let mut next_zone = from_zone.zone_in_direction(direction);
                while next_zone.is_some() {
                    let zone = next_zone.unwrap();
                    let units = state
                        .get_units_in_zone(&zone)
                        .iter()
                        .map(|c| c.get_id().clone())
                        .collect::<Vec<_>>();
                    if units.is_empty() {
                        next_zone = zone.zone_in_direction(direction);
                        continue;
                    }

                    let mut picked_unit_id = units[0];
                    if units.len() >= 1 {
                        let prompt = "Pick a unit to shoot";
                        picked_unit_id = pick_card(player_id, &units, state, prompt).await;
                    }

                    effects.push(Effect::take_damage(&picked_unit_id, shooter, *damage));
                    if splash_damage.is_some() {
                        let splash_effects = state
                            .get_units_in_zone(&zone)
                            .iter()
                            .filter(|c| c.get_id() != &picked_unit_id)
                            .map(|c| Effect::take_damage(c.get_id(), shooter, splash_damage.unwrap()))
                            .collect::<Vec<_>>();
                        effects.extend(splash_effects);
                    }

                    if !piercing {
                        break;
                    }
                    next_zone = zone.zone_in_direction(direction);
                }

                for effect in effects {
                    state.effects.push_front(effect);
                }
            }
            Effect::AddCard { card } => {
                state.cards.push(card.clone_box());
            }
            Effect::TeleportCard { card_id, to, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(to.clone());
                let effects = card.on_visit_zone(&snapshot, to).await;
                state.effects.extend(effects);
            }
            Effect::MoveCard {
                player_id,
                card_id,
                from,
                to,
                tap,
                ..
            } => {
                let snapshot = state.snapshot();
                let zone = to.resolve(player_id, state).await;
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(zone.clone());
                if *tap {
                    card.get_base_mut().tapped = true;
                }

                let card = state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                let mut effects = card.on_move(&snapshot, from, &zone).await;
                effects.extend(card.on_visit_zone(&snapshot, &zone).await);
                if let Some(site) = zone.get_site(state) {
                    effects.extend(site.on_card_enter(state, card_id));
                }

                state.effects.extend(effects);
            }
            Effect::DrawSite { player_id, count } => {
                let deck = state.decks.get_mut(player_id).unwrap();
                for _ in 0..*count {
                    let card_id = deck.sites.pop().unwrap();
                    state
                        .cards
                        .iter_mut()
                        .find(|c| c.get_id() == &card_id)
                        .unwrap()
                        .set_zone(Zone::Hand);
                }
            }
            Effect::DrawSpell { player_id, count } => {
                let deck = state.decks.get_mut(player_id).unwrap();
                for _ in 0..*count {
                    let card_id = deck.spells.pop().unwrap();
                    state
                        .cards
                        .iter_mut()
                        .find(|c| c.get_id() == &card_id)
                        .unwrap()
                        .set_zone(Zone::Hand);
                }
            }
            Effect::DrawCard { player_id, count } => {
                for _ in 0..*count {
                    let actions: Vec<Box<dyn Action>> =
                        vec![Box::new(BaseAction::DrawSite), Box::new(BaseAction::DrawSpell)];
                    let picked_action = pick_action(player_id, &actions, state, "Draw a card").await;
                    state
                        .effects
                        .extend(picked_action.on_select(None, player_id, state).await)
                }
            }
            Effect::PlayMagic { card_id, caster_id, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let mut effects = card.on_cast(&snapshot, caster_id).await;
                let mana_cost = card.get_mana_cost(&snapshot);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                    health: 0,
                });
                effects.push(Effect::MoveCard {
                    player_id: card.get_controller_id().clone(),
                    card_id: card.get_id().clone(),
                    from: card.get_zone().clone(),
                    to: ZoneQuery::Specific(Zone::Cemetery),
                    tap: false,
                    plane: Plane::None,
                });
                state.effects.extend(effects);
            }
            Effect::PlayCard { card_id, zone, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let cast_effects = card.on_summon(&snapshot);
                card.set_zone(zone.clone());
                if !card.has_modifier(&snapshot, &Modifier::Charge) {
                    card.add_modifier(Modifier::SummoningSickness);
                }

                let mut effects = card.genesis(&snapshot).await;
                effects.extend(card.on_visit_zone(&snapshot, zone).await);
                let mana_cost = card.get_mana_cost(&snapshot);
                effects.push(Effect::RemoveResources {
                    player_id: card.get_owner_id().clone(),
                    mana: mana_cost,
                    thresholds: Thresholds::new(),
                    health: 0,
                });
                state.effects.extend(effects);
                state.effects.extend(cast_effects);
            }
            Effect::SummonCard { card_id, zone, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let cast_effects = card.on_summon(&snapshot);
                card.set_zone(zone.clone());
                if !card.has_modifier(&snapshot, &Modifier::Charge) {
                    card.add_modifier(Modifier::SummoningSickness);
                }

                let mut effects = card.genesis(&snapshot).await;
                effects.extend(card.on_visit_zone(&snapshot, zone).await);
                state.effects.extend(effects);
                state.effects.extend(cast_effects);
            }
            Effect::TapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.get_base_mut().tapped = true;
            }
            Effect::StartTurn { player_id } => {
                let cards = state
                    .cards
                    .iter_mut()
                    .filter(|c| c.get_owner_id() == &state.current_player);
                for card in cards {
                    card.get_base_mut().tapped = false;
                    card.remove_modifier(Modifier::SummoningSickness);
                }

                for card in state.cards.iter().filter(|c| c.get_owner_id() == &state.current_player) {
                    let effects = card.on_turn_start(state).await;
                    state.effects.extend(effects);
                }

                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana = 0;

                let sites: Vec<&SiteBase> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_owner_id() == player_id)
                    .filter(|c| c.get_zone().is_in_realm())
                    .filter_map(|c| c.get_site_base())
                    .collect();
                for site in sites {
                    state.resources.get_mut(player_id).unwrap().mana += site.provided_mana;
                }

                let actions: Vec<Box<dyn Action>> =
                    vec![Box::new(BaseAction::DrawSite), Box::new(BaseAction::DrawSpell)];
                let prompt = "Start Turn: Pick card to draw";
                let action = pick_action(player_id, &actions, state, prompt).await;
                let effects = action.on_select(None, player_id, state).await;
                state.effects.extend(effects);
            }
            Effect::RemoveResources {
                player_id,
                mana,
                thresholds,
                health,
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana -= mana;
                player_resources.thresholds.air -= thresholds.air;
                player_resources.thresholds.water -= thresholds.water;
                player_resources.thresholds.fire -= thresholds.fire;
                player_resources.thresholds.earth -= thresholds.earth;
                player_resources.health -= health;
            }
            Effect::PreEndTurn { player_id } => {
                state.phase = Phase::PreEndTurn {
                    player_id: player_id.clone(),
                };
                for card in state.cards.iter().filter(|c| c.get_zone().is_in_realm()) {
                    let effects = card.on_turn_end(state).await;
                    state.effects.extend(effects);
                }
            }
            Effect::EndTurn { player_id } => {
                let resources = state.resources.get_mut(player_id).unwrap();
                resources.mana = 0;
                state.phase = Phase::Main;

                let cards = state.cards.iter_mut().filter(|c| c.is_unit());
                for card in cards {
                    card.get_unit_base_mut().unwrap().damage = 0;
                }
            }
            Effect::AddResources {
                player_id,
                mana,
                thresholds,
                health,
            } => {
                let player_resources = state.resources.get_mut(player_id).unwrap();
                player_resources.mana += mana;
                player_resources.thresholds.air += thresholds.air;
                player_resources.thresholds.water += thresholds.water;
                player_resources.thresholds.fire += thresholds.fire;
                player_resources.thresholds.earth += thresholds.earth;
                player_resources.health += health;
            }
            Effect::Attack {
                attacker_id,
                defender_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.cards.iter().find(|c| c.get_id() == attacker_id).unwrap();
                let defender = state.cards.iter().find(|c| c.get_id() == defender_id).unwrap();
                let mut effects = vec![
                    Effect::MoveCard {
                        player_id: attacker.get_controller_id().clone(),
                        card_id: attacker_id.clone(),
                        from: attacker.get_zone().clone(),
                        to: ZoneQuery::Specific(defender.get_zone().clone()),
                        tap: true,
                        plane: attacker.get_base().plane.clone(),
                    },
                    Effect::TakeDamage {
                        card_id: defender_id.clone(),
                        from: attacker_id.clone(),
                        damage: attacker.get_power(&snapshot).unwrap(),
                    },
                ];
                effects.extend(attacker.after_attack(state).await);
                state.effects.extend(effects);
                state.effects.extend(defender.on_defend(state, attacker_id));
            }
            Effect::DealDamageToTarget {
                player_id,
                query,
                from,
                damage,
            } => {
                let target = query.resolve(player_id, state).await;
                state.effects.push_front(Effect::TakeDamage {
                    card_id: target,
                    from: from.clone(),
                    damage: *damage,
                });
            }
            Effect::TakeDamage { card_id, damage, from } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let effects = card.on_take_damage(&snapshot, from, *damage);
                for effect in effects {
                    state.effects.push_front(effect);
                }
            }
            Effect::BanishCard { card_id, .. } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(Zone::Banish);
            }
            Effect::BuryCard { card_id, from } => {
                {
                    let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                    card.set_zone(Zone::Cemetery);
                }

                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                let effects = card.deathrite(&snapshot, from);
                state.effects.extend(effects);
            }
            Effect::AddCounter { card_id, counter } => {
                let card = state.get_card_mut(card_id).unwrap();
                if card.is_unit() {
                    let base = card.get_unit_base_mut().unwrap();
                    base.power_counters.push(counter.clone());
                }
            }
            Effect::AddModifier { card_id, counter } => {
                let card = state.get_card_mut(card_id).unwrap();
                if card.is_unit() {
                    let base = card.get_unit_base_mut().unwrap();
                    base.modifier_counters.push(counter.clone());
                }
            }
            Effect::SetCardData { card_id, data } => {
                let card = state.get_card_mut(card_id).unwrap();
                card.set_data(data)?;
            }
            Effect::RangedStrike {
                attacker_id,
                defender_id,
            } => {
                let snapshot = state.snapshot();
                let attacker = state.cards.iter().find(|c| c.get_id() == attacker_id).unwrap();
                let defender = state.cards.iter().find(|c| c.get_id() == defender_id).unwrap();
                let mut effects = vec![Effect::TakeDamage {
                    card_id: defender_id.clone(),
                    from: attacker_id.clone(),
                    damage: attacker.get_power(&snapshot).unwrap(),
                }];
                effects.extend(attacker.after_attack(state).await);
                state.effects.extend(effects);
                state.effects.extend(defender.on_defend(state, attacker_id));
            }
            Effect::TeleportUnitToZone {
                player_id,
                unit_query,
                zone_query,
            } => {
                let unit_id = unit_query.resolve(player_id, state).await;
                let unit = state.get_card(&unit_id).unwrap();
                let zone = zone_query.resolve(player_id, state).await;
                state.effects.push_back(Effect::MoveCard {
                    player_id: player_id.clone(),
                    card_id: unit_id.clone(),
                    from: unit.get_zone().clone(),
                    to: ZoneQuery::Specific(zone),
                    tap: false,
                    plane: Plane::Surface,
                });
            }
            Effect::RearrangeDeck { spells, sites } => {
                let deck = state.decks.get_mut(&state.current_player).unwrap();
                deck.spells = spells.clone();
                deck.sites = sites.clone();
            }
            Effect::Burrow { card_id } => {
                let card = state.get_card_mut(card_id).unwrap();
                card.get_base_mut().plane = Plane::Burrowed;
            }
            Effect::Submerge { card_id } => {
                let card = state.get_card_mut(card_id).unwrap();
                card.get_base_mut().plane = Plane::Submerged;
            }
        }

        self.expire_counters(state).await;

        Ok(())
    }
}
