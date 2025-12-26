use crate::{
    card::{Card, Modifier, Plane, SiteBase, UnitBase, Zone},
    game::{Action, BaseAction, Direction, PlayerId, Thresholds, pick_action, pick_card, pick_zone},
    state::{Phase, State},
};
use rand::seq::IndexedRandom;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum EffectQuery {
    EnterZone { card: CardQuery, zone: ZoneQuery },
    TurnEnd,
}

impl EffectQuery {
    pub fn matches(&self, effect: &Effect, state: &State) -> bool {
        match (self, effect) {
            (EffectQuery::EnterZone { card, zone }, Effect::MoveCard { card_id, to, .. }) => {
                let cards = card.options(state);
                let zones = zone.options(state);
                return cards.contains(card_id) && zones.contains(to);
            }
            (EffectQuery::TurnEnd, Effect::EndTurn { .. }) => true,
            _ => false,
        }
    }
}

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

#[derive(Debug, Clone)]
pub enum CardQuery {
    Specific(uuid::Uuid),
    InZone { zone: Zone, owner: Option<PlayerId> },
    NearZone { zone: Zone, owner: Option<PlayerId> },
    OwnedBy { owner: uuid::Uuid },
    RandomUnitInZone { zone: ZoneQuery },
}

impl CardQuery {
    pub fn options(&self, state: &State) -> Vec<uuid::Uuid> {
        match self {
            CardQuery::Specific(id) => vec![id.clone()],
            CardQuery::InZone { zone, owner } => zone
                .get_units(state, owner.as_ref())
                .iter()
                .map(|c| c.get_id().clone())
                .collect(),
            CardQuery::NearZone { zone, owner } => zone
                .get_nearby_units(state, owner.as_ref())
                .iter()
                .map(|c| c.get_id().clone())
                .collect(),
            CardQuery::OwnedBy { owner } => state
                .cards
                .iter()
                .filter(|c| c.get_owner_id() == owner)
                .map(|c| c.get_id().clone())
                .collect(),
            _ => unreachable!(),
        }
    }

    pub async fn resolve(&self, player_id: &PlayerId, state: &State, prompt: &str) -> uuid::Uuid {
        match self {
            CardQuery::Specific(id) => id.clone(),
            CardQuery::InZone { zone, owner } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_units(state, owner.as_ref())
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt).await
            }
            CardQuery::NearZone { zone, owner } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_nearby_units(state, owner.as_ref())
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt).await
            }
            CardQuery::OwnedBy { owner } => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == owner)
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt).await
            }
            CardQuery::RandomUnitInZone { zone } => {
                let zone = zone.resolve(player_id, state, prompt).await;
                let cards: Vec<uuid::Uuid> = state
                    .get_units_in_zone(&zone)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                cards.choose(&mut rand::rng()).unwrap().clone()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZoneQuery {
    Any,
    AnySite,
    Specific(Zone),
}

impl ZoneQuery {
    pub fn options(&self, state: &State) -> Vec<Zone> {
        match self {
            ZoneQuery::Any => Zone::all_realm(),
            ZoneQuery::Specific(z) => vec![z.clone()],
            ZoneQuery::AnySite => {
                let mut sites = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                sites
            }
        }
    }
    pub async fn resolve(&self, player_id: &PlayerId, state: &State, prompt: &str) -> Zone {
        match self {
            ZoneQuery::Any => pick_zone(player_id, &Zone::all_realm(), state, prompt).await,
            ZoneQuery::Specific(z) => z.clone(),
            ZoneQuery::AnySite => {
                let mut sites = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                pick_zone(player_id, &sites, state, prompt).await
            }
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
        prompt: String,
    },
    DealDamageToTarget {
        player_id: uuid::Uuid,
        query: CardQuery,
        from: uuid::Uuid,
        damage: u8,
        prompt: String,
    },
    RearrangeDeck {
        spells: Vec<uuid::Uuid>,
        sites: Vec<uuid::Uuid>,
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
            Effect::SummonCard { .. } => "SummonCard".to_string(),
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
            Effect::TeleportUnitToZone { .. } => "TeleportUnitToZone".to_string(),
            Effect::RearrangeDeck { .. } => "RearrangeDeck".to_string(),
            Effect::Burrow { .. } => "Burrow".to_string(),
            Effect::Submerge { .. } => "Submerge".to_string(),
        }
    }

    fn expire_counters(&self, state: &mut State) {
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
                    if effect_query.matches(self, state) {
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
                    if effect_query.matches(self, state) {
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
                if *tap {
                    card.get_base_mut().tapped = true;
                }

                let card = state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                let mut effects = card.on_move(&snapshot, from, to).await;
                effects.extend(card.on_visit_zone(&snapshot, to).await);
                if let Some(site) = to.get_site(state) {
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
                let target = query.resolve(player_id, state, prompt).await;
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
                prompt,
            } => {
                let unit_id = unit_query.resolve(player_id, state, prompt).await;
                let unit = state.get_card(&unit_id).unwrap();
                let zone = zone_query.resolve(player_id, state, prompt).await;
                state.effects.push_back(Effect::MoveCard {
                    card_id: unit_id.clone(),
                    from: unit.get_zone().clone(),
                    to: zone,
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

        self.expire_counters(state);

        Ok(())
    }
}
