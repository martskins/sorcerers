use std::{collections::HashMap, sync::OnceLock};

use rand::seq::IndexedRandom;
use tokio::sync::RwLock;

use crate::{
    card::Zone,
    effect::Effect,
    game::{PlayerId, pick_card, pick_card_with_preview, pick_zone},
    state::State,
};

static QUERY_CACHE: OnceLock<RwLock<QueryCache>> = OnceLock::new();

#[derive(Debug)]
pub struct QueryCache {
    zone_queries: HashMap<uuid::Uuid, Zone>,
    card_queries: HashMap<uuid::Uuid, uuid::Uuid>,
    game_queries: HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
}

impl QueryCache {
    fn new() -> Self {
        Self {
            zone_queries: HashMap::new(),
            card_queries: HashMap::new(),
            game_queries: HashMap::new(),
        }
    }

    pub fn init() {
        QUERY_CACHE.get_or_init(|| RwLock::new(QueryCache::new()));
    }

    pub async fn resolve_card(qry: &CardQuery, player_id: &PlayerId, state: &State) -> uuid::Uuid {
        if let Some(cached) = QUERY_CACHE.get().unwrap().read().await.card_queries.get(&qry.get_id()) {
            return cached.clone();
        }

        let card_id = match qry {
            CardQuery::Specific { card_id, .. } => card_id.clone(),
            CardQuery::InZone {
                zone, owner, prompt, ..
            } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_units(state, owner.as_ref())
                    .iter()
                    .filter(|c| c.can_be_targetted_by(state, player_id))
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a card", |v| v)).await
            }
            CardQuery::NearZone {
                zone, owner, prompt, ..
            } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_nearby_units(state, owner.as_ref())
                    .iter()
                    .filter(|c| c.can_be_targetted_by(state, player_id))
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a card", |v| v)).await
            }
            CardQuery::OwnedBy { owner, prompt, .. } => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == owner)
                    .filter(|c| c.can_be_targetted_by(state, player_id))
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a card", |v| v)).await
            }
            CardQuery::RandomTarget { possible_targets, .. } => {
                for card in &state.cards {
                    if let Some(query) = card.card_query_override(state, qry) {
                        return Box::pin(query.resolve(player_id, state)).await;
                    }
                }

                possible_targets.choose(&mut rand::rng()).unwrap().clone()
            }
            CardQuery::RandomUnitInZone { zone, .. } => {
                for card in &state.cards {
                    if let Some(query) = card.card_query_override(state, qry) {
                        return Box::pin(query.resolve(player_id, state)).await;
                    }
                }

                let cards: Vec<uuid::Uuid> = state
                    .get_units_in_zone(&zone)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                cards.choose(&mut rand::rng()).unwrap().clone()
            }
            CardQuery::FromOptions {
                options,
                prompt,
                preview,
                ..
            } => {
                if *preview {
                    return pick_card_with_preview(
                        player_id,
                        options,
                        state,
                        prompt.as_ref().unwrap_or(&String::new()),
                    )
                    .await;
                }

                pick_card(player_id, &options, state, prompt.as_ref().unwrap_or(&String::new())).await
            }
        };

        let mut cache = QUERY_CACHE.get().unwrap().write().await;
        cache.card_queries.insert(qry.get_id().clone(), card_id.clone());
        cache
            .game_queries
            .entry(state.game_id)
            .or_insert(Vec::new())
            .push(qry.get_id().clone());

        card_id
    }

    pub async fn resolve_zone(qry: &ZoneQuery, player_id: &PlayerId, state: &State) -> Zone {
        if let Some(cached) = QUERY_CACHE.get().unwrap().read().await.zone_queries.get(&qry.get_id()) {
            return cached.clone();
        }

        let zone = match qry {
            ZoneQuery::Any { prompt, .. } => {
                pick_zone(
                    player_id,
                    &Zone::all_realm(),
                    state,
                    prompt.as_ref().map_or("Pick a zone", |v| v),
                )
                .await
            }
            ZoneQuery::Specific { zone, .. } => zone.clone(),
            ZoneQuery::AnySite { prompt, .. } => {
                let mut sites = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone().is_in_realm())
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                pick_zone(player_id, &sites, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
            ZoneQuery::Random { options, .. } => {
                for card in &state.cards {
                    if let Some(query) = card.zone_query_override(state, qry) {
                        return Box::pin(query.resolve(player_id, state)).await;
                    }
                }

                options.choose(&mut rand::rng()).unwrap().clone()
            }
            ZoneQuery::FromOptions { options, prompt, .. } => {
                pick_zone(player_id, options, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
        };

        let mut cache = QUERY_CACHE.get().unwrap().write().await;
        cache.zone_queries.insert(qry.get_id().clone(), zone.clone());
        cache
            .game_queries
            .entry(state.game_id)
            .or_insert(Vec::new())
            .push(qry.get_id().clone());

        zone
    }

    pub async fn clear_game_cache(game_id: &uuid::Uuid) {
        if let Some(cache) = QUERY_CACHE.get() {
            let mut cache = cache.write().await;
            if let Some(queries) = cache.game_queries.remove(game_id) {
                for qry_id in queries {
                    cache.zone_queries.remove(&qry_id);
                    cache.card_queries.remove(&qry_id);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZoneQuery {
    Any {
        id: uuid::Uuid,
        prompt: Option<String>,
    },
    AnySite {
        id: uuid::Uuid,
        controlled_by: Option<PlayerId>,
        prompt: Option<String>,
    },
    Specific {
        id: uuid::Uuid,
        zone: Zone,
    },
    Random {
        id: uuid::Uuid,
        options: Vec<Zone>,
    },
    FromOptions {
        id: uuid::Uuid,
        options: Vec<Zone>,
        prompt: Option<String>,
    },
}

impl ZoneQuery {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            ZoneQuery::Any { id, .. } => id,
            ZoneQuery::Specific { id, .. } => id,
            ZoneQuery::AnySite { id, .. } => id,
            ZoneQuery::Random { id, .. } => id,
            ZoneQuery::FromOptions { id, .. } => id,
        }
    }

    pub fn prompt(&self) -> &str {
        match self {
            ZoneQuery::Any { prompt, .. } => {
                if let Some(p) = prompt {
                    p.as_ref()
                } else {
                    "Pick a zone"
                }
            }
            ZoneQuery::Specific { .. } => "Pick a zone",
            ZoneQuery::AnySite { prompt, .. } => {
                if let Some(p) = prompt {
                    p.as_ref()
                } else {
                    "Pick a site zone"
                }
            }
            ZoneQuery::Random { .. } => "Pick a zone",
            ZoneQuery::FromOptions { prompt, .. } => {
                if let Some(p) = prompt {
                    p.as_ref()
                } else {
                    "Pick a zone"
                }
            }
        }
    }

    pub fn options(&self, state: &State) -> Vec<Zone> {
        match self {
            ZoneQuery::Any { .. } => Zone::all_realm(),
            ZoneQuery::Specific { zone, .. } => vec![zone.clone()],
            ZoneQuery::AnySite { .. } => {
                let mut sites = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone().is_in_realm())
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                sites
            }
            ZoneQuery::Random { options, .. } => {
                vec![options.choose(&mut rand::rng()).unwrap().clone()]
            }
            ZoneQuery::FromOptions { options, .. } => options.clone(),
        }
    }

    pub async fn resolve(&self, player_id: &PlayerId, state: &State) -> Zone {
        QueryCache::resolve_zone(self, player_id, state).await
    }
}

#[derive(Debug, Clone)]
pub enum CardQuery {
    Specific {
        id: uuid::Uuid,
        card_id: uuid::Uuid,
    },
    InZone {
        id: uuid::Uuid,
        zone: Zone,
        owner: Option<PlayerId>,
        prompt: Option<String>,
    },
    NearZone {
        id: uuid::Uuid,
        zone: Zone,
        owner: Option<PlayerId>,
        prompt: Option<String>,
    },
    OwnedBy {
        id: uuid::Uuid,
        owner: uuid::Uuid,
        prompt: Option<String>,
    },
    RandomUnitInZone {
        id: uuid::Uuid,
        zone: Zone,
    },
    RandomTarget {
        id: uuid::Uuid,
        possible_targets: Vec<uuid::Uuid>,
    },
    FromOptions {
        id: uuid::Uuid,
        options: Vec<uuid::Uuid>,
        prompt: Option<String>,
        preview: bool,
    },
}

impl CardQuery {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            CardQuery::Specific { id, .. } => id,
            CardQuery::InZone { id, .. } => id,
            CardQuery::NearZone { id, .. } => id,
            CardQuery::OwnedBy { id, .. } => id,
            CardQuery::RandomUnitInZone { id, .. } => id,
            CardQuery::RandomTarget { id, .. } => id,
            CardQuery::FromOptions { id, .. } => id,
        }
    }

    pub fn options(&self, state: &State) -> Vec<uuid::Uuid> {
        match self {
            CardQuery::Specific { card_id, .. } => vec![card_id.clone()],
            CardQuery::InZone { zone, owner, .. } => zone
                .get_units(state, owner.as_ref())
                .iter()
                .map(|c| c.get_id().clone())
                .collect(),
            CardQuery::NearZone { zone, owner, .. } => zone
                .get_nearby_units(state, owner.as_ref())
                .iter()
                .map(|c| c.get_id().clone())
                .collect(),
            CardQuery::OwnedBy { owner, .. } => state
                .cards
                .iter()
                .filter(|c| c.get_owner_id() == owner)
                .map(|c| c.get_id().clone())
                .collect(),
            CardQuery::FromOptions { options, .. } => options.clone(),
            CardQuery::RandomUnitInZone { .. } => unreachable!(),
            CardQuery::RandomTarget { .. } => unreachable!(),
        }
    }

    pub async fn resolve(&self, player_id: &PlayerId, state: &State) -> uuid::Uuid {
        QueryCache::resolve_card(self, player_id, state).await
    }
}

#[derive(Debug, Clone)]
pub enum EffectQuery {
    EnterZone { card: CardQuery, zone: ZoneQuery },
    TurnEnd,
}

impl EffectQuery {
    pub async fn matches(&self, effect: &Effect, state: &State) -> bool {
        match (self, effect) {
            (
                EffectQuery::EnterZone { card, zone },
                Effect::MoveCard {
                    player_id, card_id, to, ..
                },
            ) => {
                let cards = card.options(state);
                let zones = zone.options(state);
                let zone = to.resolve(player_id, state).await;
                return cards.contains(card_id) && zones.contains(&zone);
            }
            (EffectQuery::TurnEnd, Effect::EndTurn { .. }) => true,
            _ => false,
        }
    }
}
