use crate::{
    card::{Modifier, Plane, SiteBase, UnitBase, Zone},
    game::{Action, BaseAction, Direction, PlayerId, Thresholds, pick_action, pick_card},
    state::{Phase, State},
};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct ModifierCounter {
    pub modifier: Modifier,
    pub expires_in_turns: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub power: i8,
    pub toughness: i8,
    pub expires_in_turns: Option<u8>,
}

impl Counter {
    pub fn new(power: i8, toughness: i8, expires_in_turns: Option<u8>) -> Self {
        Self {
            power,
            toughness,
            expires_in_turns,
        }
    }
}

#[derive(Debug)]
pub enum Query {
    InZone { zone: Zone, owner: Option<PlayerId> },
    NearZone { zone: Zone, owner: Option<PlayerId> },
    OwnedBy { owner: uuid::Uuid },
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
    MoveCard {
        card_id: uuid::Uuid,
        from: Zone,
        to: Zone,
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
    DealDamageToTarget {
        player_id: uuid::Uuid,
        query: Query,
        from: uuid::Uuid,
        damage: u8,
        prompt: String,
    },
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

    pub fn add_modifier(card_id: &uuid::Uuid, modifier: Modifier, expires_in_turns: Option<u8>) -> Self {
        Effect::AddModifier {
            card_id: card_id.clone(),
            counter: ModifierCounter {
                modifier,
                expires_in_turns,
            },
        }
    }

    pub fn name(&self, state: &State) -> String {
        match self {
            Effect::ShootProjectile { .. } => "ShootProjectile".to_string(),
            Effect::AddCard { .. } => "AddCard".to_string(),
            Effect::AddModifier { .. } => "AddModifier".to_string(),
            Effect::AddCounter { .. } => "AddCounter".to_string(),
            Effect::TeleportCard { .. } => "TeleportCard".to_string(),
            Effect::MoveCard { .. } => "MoveCard".to_string(),
            Effect::DrawCard { .. } => "DrawCard".to_string(),
            Effect::DrawSite { .. } => "DrawSite".to_string(),
            Effect::DrawSpell { .. } => "DrawSpell".to_string(),
            Effect::PlayMagic { .. } => "PlayMagic".to_string(),
            Effect::PlayCard { .. } => "PlayCard".to_string(),
            Effect::TapCard { .. } => "TapCard".to_string(),
            Effect::PreEndTurn { .. } => "PrepareEndTurn".to_string(),
            Effect::EndTurn { .. } => "EndTurn".to_string(),
            Effect::StartTurn { .. } => "StartTurn".to_string(),
            Effect::RemoveResources { .. } => "RemoveResources".to_string(),
            Effect::AddResources { .. } => "AddResources".to_string(),
            Effect::Attack { .. } => "Attack".to_string(),
            Effect::TakeDamage { card_id, from, damage } => {
                let attacker = state.get_card(from).map_or("Unknown", |c| c.get_name());
                let defender = state.get_card(card_id).map_or("Unknown", |c| c.get_name());
                format!("TakeDamage: {} deals {} damage to {}", attacker, damage, defender)
            }
            Effect::BuryCard { .. } => "BuryCard".to_string(),
            Effect::BanishCard { .. } => "BanishCard".to_string(),
            Effect::SetCardData { .. } => "SetCardData".to_string(),
            Effect::RangedStrike { .. } => "RangedStrike".to_string(),
            Effect::DealDamageToTarget { .. } => "DealDamageToTarget".to_string(),
        }
    }

    pub async fn apply(&self, state: &mut State) -> anyhow::Result<()> {
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
                card_id, from, to, tap, ..
            } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(to.clone());
                let mut effects = card.on_move(&snapshot, from, to).await;
                effects.extend(card.on_visit_zone(&snapshot, to).await);
                state.effects.extend(effects);
                if *tap {
                    card.get_base_mut().tapped = true;
                }
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
                    card_id: card.get_id().clone(),
                    from: card.get_zone().clone(),
                    to: Zone::Cemetery,
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
                if !card.has_modifier(&snapshot, Modifier::Charge) {
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
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
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
                for card in state.cards.iter().filter(|c| matches!(c.get_zone(), Zone::Realm(_))) {
                    let effects = card.on_turn_end(state).await;
                    state.effects.extend(effects);
                }
            }
            Effect::EndTurn { player_id } => {
                let resources = state.resources.get_mut(player_id).unwrap();
                resources.mana = 0;

                // Clear any counters that expire at the end of the turn
                let card_ids: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.is_unit())
                    .filter(|c| {
                        !c.get_unit_base()
                            .unwrap_or(&UnitBase::default())
                            .power_counters
                            .is_empty()
                    })
                    .map(|c| c.get_id().clone())
                    .collect();
                for card_id in card_ids {
                    let card = state.get_card_mut(&card_id).unwrap();
                    let base = card.get_unit_base_mut().unwrap();
                    for counter in &mut base.power_counters {
                        if counter.expires_in_turns.is_some() && counter.expires_in_turns.unwrap() > 0 {
                            counter.expires_in_turns = Some(counter.expires_in_turns.unwrap() - 1);
                        }
                    }

                    card.get_unit_base_mut()
                        .unwrap()
                        .power_counters
                        .retain(|c| c.expires_in_turns.is_none() || c.expires_in_turns.unwrap() > 0);
                }
                state.phase = Phase::Main;
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
                        card_id: attacker_id.clone(),
                        from: attacker.get_zone().clone(),
                        to: defender.get_zone().clone(),
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
                prompt,
            } => {
                let cards: Vec<uuid::Uuid> = match query {
                    Query::InZone { zone, owner } => zone
                        .get_units(state, owner.as_ref())
                        .iter()
                        .map(|c| c.get_id().clone())
                        .collect(),
                    Query::NearZone { zone, owner } => zone
                        .get_nearby_units(state, owner.as_ref())
                        .iter()
                        .map(|c| c.get_id().clone())
                        .collect(),
                    Query::OwnedBy { owner } => state
                        .cards
                        .iter()
                        .filter(|c| c.get_owner_id() == owner)
                        .map(|c| c.get_id().clone())
                        .collect(),
                };
                let picked_card = pick_card(player_id, &cards, state, prompt).await;
                state.effects.push_front(Effect::TakeDamage {
                    card_id: picked_card,
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
                // TODO: Review this
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
        }

        Ok(())
    }
}
