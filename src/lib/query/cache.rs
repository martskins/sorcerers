use rand::seq::IndexedRandom;

use crate::{
    game::{CardId, PlayerId, pick_location_source},
    query::ZoneQuery,
    state::State,
    zone::{Location, Zone},
};
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

static QUERY_CACHE: OnceLock<RwLock<QueryCache>> = OnceLock::new();

#[derive(Debug)]
pub struct QueryCache {
    zone_queries: HashMap<uuid::Uuid, Zone>,
    card_queries: HashMap<uuid::Uuid, uuid::Uuid>,
    effect_targets: HashMap<uuid::Uuid, Vec<CardId>>,
    game_queries: HashMap<uuid::Uuid, Vec<CardId>>,
}

impl QueryCache {
    fn new() -> Self {
        Self {
            zone_queries: HashMap::new(),
            card_queries: HashMap::new(),
            effect_targets: HashMap::new(),
            game_queries: HashMap::new(),
        }
    }

    pub fn init() {
        QUERY_CACHE.get_or_init(|| RwLock::new(QueryCache::new()));
    }

    pub fn card_result(query_id: &uuid::Uuid) -> Option<CardId> {
        let cache = QUERY_CACHE.get().expect("to get lock").read().unwrap();
        cache.card_queries.get(query_id).cloned()
    }

    pub fn store_card_result(game_id: uuid::Uuid, query_id: uuid::Uuid, card_id: CardId) {
        let mut cache = QUERY_CACHE.get().expect("to get lock").write().unwrap();
        cache.card_queries.insert(query_id, card_id);

        cache
            .game_queries
            .entry(game_id)
            .or_default()
            .push(query_id);
    }

    pub fn store_effect_targets(
        game_id: uuid::Uuid,
        effect_id: uuid::Uuid,
        affected_cards: Vec<CardId>,
    ) {
        let mut cache = QUERY_CACHE.get().expect("to get lock").write().unwrap();
        cache.effect_targets.insert(effect_id, affected_cards);

        cache
            .game_queries
            .entry(game_id)
            .or_default()
            .push(effect_id);
    }

    pub fn effect_targets(effect_id: &uuid::Uuid) -> Option<Vec<CardId>> {
        let cache = QUERY_CACHE.get().expect("to get lock").read().unwrap();
        cache.effect_targets.get(effect_id).cloned()
    }

    pub async fn resolve_zone(
        qry: &ZoneQuery,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Zone> {
        if let Some(cached) = QUERY_CACHE
            .get()
            .expect("lock to be obtained")
            .read()
            .unwrap()
            .zone_queries
            .get(&qry.id)
        {
            return Ok(cached.clone());
        }

        if qry.allow_modifiers && qry.zone.is_none() {
            for effect in state.active_continuous_effects() {
                if let crate::state::OngoingEffect::ModifyZoneQuery { modifier, .. } = effect
                    && let Some(query) = modifier(state, player_id, qry)?
                {
                    return Box::pin(query.without_modifiers().pick(player_id, state)).await;
                }
            }
        }

        let zone = if let Some(zone) = &qry.zone {
            zone.clone()
        } else if qry.random {
            let options = qry.options(state);
            options
                .as_slice()
                .choose(&mut rand::rng())
                .expect("failed to get random zone")
                .clone()
        } else if let Some(options) = &qry.options {
            Zone::Location(
                pick_location_source(
                    player_id,
                    &options
                        .iter()
                        .filter_map(Zone::location)
                        .cloned()
                        .collect::<Vec<_>>(),
                    state,
                    false,
                    qry.prompt(),
                    qry.source_card_id,
                )
                .await?,
            )
        } else if qry.sites_only {
            let mut sites: Vec<Location> = state
                .cards
                .values()
                .filter(|c| c.is_site())
                .filter(|c| c.get_zone().is_in_play())
                .filter(|c| {
                    qry.controlled_by
                        .as_ref()
                        .is_none_or(|p| c.get_controller_id(state) == *p)
                })
                .filter_map(|c| c.get_zone().location().cloned())
                .collect();
            sites.dedup();
            Zone::Location(
                pick_location_source(
                    player_id,
                    &sites,
                    state,
                    false,
                    qry.prompt(),
                    qry.source_card_id,
                )
                .await?,
            )
        } else {
            Zone::Location(
                pick_location_source(
                    player_id,
                    &Zone::all_realm()
                        .iter()
                        .filter_map(Zone::location)
                        .cloned()
                        .collect::<Vec<_>>(),
                    state,
                    false,
                    qry.prompt(),
                    qry.source_card_id,
                )
                .await?,
            )
        };

        let mut cache = QUERY_CACHE
            .get()
            .expect("failed to get random zone")
            .write()
            .unwrap();
        cache.zone_queries.insert(qry.id, zone.clone());
        cache
            .game_queries
            .entry(state.game_id)
            .or_default()
            .push(qry.id);

        Ok(zone)
    }

    pub fn clear_game_cache(game_id: &uuid::Uuid) {
        if let Some(cache) = QUERY_CACHE.get() {
            let mut cache = cache.write().unwrap();
            if let Some(queries) = cache.game_queries.remove(game_id) {
                for qry_id in queries {
                    cache.zone_queries.remove(&qry_id);
                    cache.card_queries.remove(&qry_id);
                    cache.effect_targets.remove(&qry_id);
                }
            }
        }
    }
}
