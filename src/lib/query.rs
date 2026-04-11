use crate::{
    card::Zone,
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};
use rand::seq::IndexedRandom;
use std::{collections::HashMap, sync::OnceLock};
use tokio::sync::RwLock;

static QUERY_CACHE: OnceLock<RwLock<QueryCache>> = OnceLock::new();

#[derive(Debug)]
pub struct QueryCache {
    zone_queries: HashMap<uuid::Uuid, Zone>,
    card_queries: HashMap<uuid::Uuid, uuid::Uuid>,
    effect_targets: HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
    matcher_queries: HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
    game_queries: HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
}

impl QueryCache {
    fn new() -> Self {
        Self {
            zone_queries: HashMap::new(),
            card_queries: HashMap::new(),
            game_queries: HashMap::new(),
            effect_targets: HashMap::new(),
            matcher_queries: HashMap::new(),
        }
    }

    pub fn init() {
        QUERY_CACHE.get_or_init(|| RwLock::new(QueryCache::new()));
    }

    pub async fn store_matcher_results(game_id: uuid::Uuid, matcher_id: uuid::Uuid, card_ids: Vec<uuid::Uuid>) {
        let mut cache = QUERY_CACHE.get().expect("to get lock").write().await;
        cache.matcher_queries.insert(matcher_id, card_ids);
        cache.game_queries.entry(game_id).or_insert(Vec::new()).push(matcher_id);
    }

    pub async fn matcher_results(matcher_id: &uuid::Uuid) -> Option<Vec<uuid::Uuid>> {
        let cache = QUERY_CACHE.get().expect("to get lock").read().await;
        cache.matcher_queries.get(matcher_id).cloned()
    }

    pub async fn store_effect_targets(game_id: uuid::Uuid, effect_id: uuid::Uuid, affected_cards: Vec<uuid::Uuid>) {
        let mut cache = QUERY_CACHE.get().expect("to get lock").write().await;
        cache.effect_targets.insert(effect_id.clone(), affected_cards);

        cache
            .game_queries
            .entry(game_id)
            .or_insert(Vec::new())
            .push(effect_id.clone());
    }

    pub async fn effect_targets(effect_id: &uuid::Uuid) -> Option<Vec<uuid::Uuid>> {
        let cache = QUERY_CACHE.get().expect("to get lock").read().await;
        cache.effect_targets.get(effect_id).cloned()
    }

    pub async fn resolve_zone(qry: &ZoneQuery, player_id: &PlayerId, state: &State) -> anyhow::Result<Zone> {
        if let Some(cached) = QUERY_CACHE
            .get()
            .expect("lock to be obtained")
            .read()
            .await
            .zone_queries
            .get(&qry.get_id())
        {
            return Ok(cached.clone());
        }

        let zone = match qry {
            ZoneQuery::Any { prompt, .. } => {
                pick_zone(
                    player_id,
                    &Zone::all_realm(),
                    state,
                    false,
                    prompt.as_ref().map_or("Pick a zone", |v| v),
                )
                .await?
            }
            ZoneQuery::Specific { zone, .. } => zone.clone(),
            ZoneQuery::AnySite { prompt, .. } => {
                let mut sites = state
                    .cards
                    .iter()
                    .filter(|c| c.is_site())
                    .filter(|c| c.get_zone().is_in_play())
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                pick_zone(
                    player_id,
                    &sites,
                    state,
                    false,
                    prompt.as_ref().map_or("Pick a zone", |v| v),
                )
                .await?
            }
            ZoneQuery::Random { options, .. } => {
                for card in &state.cards {
                    if let Some(query) = card.zone_query_override(state, qry)? {
                        return Ok(Box::pin(query.resolve(player_id, state)).await?);
                    }
                }

                options
                    .choose(&mut rand::rng())
                    .expect("failed to get random zone")
                    .clone()
            }
            ZoneQuery::FromOptions { options, prompt, .. } => {
                pick_zone(
                    player_id,
                    options,
                    state,
                    false,
                    prompt.as_ref().map_or("Pick a zone", |v| v),
                )
                .await?
            }
        };

        let mut cache = QUERY_CACHE.get().expect("failed to get random zone").write().await;
        cache.zone_queries.insert(qry.get_id().clone(), zone.clone());
        cache
            .game_queries
            .entry(state.game_id)
            .or_insert(Vec::new())
            .push(qry.get_id().clone());

        Ok(zone)
    }

    pub async fn clear_game_cache(game_id: &uuid::Uuid) {
        if let Some(cache) = QUERY_CACHE.get() {
            let mut cache = cache.write().await;
            if let Some(queries) = cache.game_queries.remove(game_id) {
                for qry_id in queries {
                    cache.zone_queries.remove(&qry_id);
                    cache.card_queries.remove(&qry_id);
                    cache.effect_targets.remove(&qry_id);
                    cache.matcher_queries.remove(&qry_id);
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

impl Into<ZoneQuery> for &Zone {
    fn into(self) -> ZoneQuery {
        ZoneQuery::Specific {
            id: uuid::Uuid::new_v4(),
            zone: self.clone(),
        }
    }
}

impl Into<ZoneQuery> for Zone {
    fn into(self) -> ZoneQuery {
        ZoneQuery::Specific {
            id: uuid::Uuid::new_v4(),
            zone: self,
        }
    }
}

impl ZoneQuery {
    pub fn from_zone(zone: Zone) -> Self {
        ZoneQuery::Specific {
            id: uuid::Uuid::new_v4(),
            zone,
        }
    }

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
                    .filter(|c| c.get_zone().is_in_play())
                    .map(|c| c.get_zone().clone())
                    .collect::<Vec<Zone>>();
                sites.dedup();
                sites
            }
            ZoneQuery::Random { options, .. } => {
                vec![
                    options
                        .choose(&mut rand::rng())
                        .expect("failed to get random zone")
                        .clone(),
                ]
            }
            ZoneQuery::FromOptions { options, .. } => options.clone(),
        }
    }

    pub async fn resolve(&self, player_id: &PlayerId, state: &State) -> anyhow::Result<Zone> {
        QueryCache::resolve_zone(self, player_id, state).await
    }
}

#[derive(Debug, Clone)]
pub enum EffectQuery {
    EnterZone {
        card: CardQuery,
        zone: ZoneQuery,
    },
    DamageDealt {
        source: Option<CardQuery>,
        target: Option<CardQuery>,
    },
    TurnEnd {
        player_id: Option<PlayerId>,
    },
    TurnStart {
        player_id: Option<PlayerId>,
    },
    MoveCard {
        card: CardQuery,
    },
    PlayCard {
        card: CardQuery,
    },
    SummonCard {
        card: CardQuery,
    },
}

impl EffectQuery {
    pub async fn matches(&self, effect: &Effect, state: &State) -> anyhow::Result<bool> {
        match (self, effect) {
            (
                EffectQuery::EnterZone { card, zone },
                Effect::MoveCard {
                    player_id, card_id, to, ..
                },
            ) => {
                let zones = zone.options(state);
                let zone = to.resolve(player_id, state).await?;
                Ok(card.matches(card_id, state) && zones.contains(&zone))
            }
            (
                EffectQuery::TurnStart {
                    player_id: query_player_id,
                },
                Effect::StartTurn {
                    player_id: effect_player_id,
                    ..
                },
            ) => {
                if let Some(query_player_id) = query_player_id {
                    Ok(query_player_id == effect_player_id)
                } else {
                    Ok(true)
                }
            }
            (
                EffectQuery::TurnEnd {
                    player_id: query_player_id,
                },
                Effect::EndTurn {
                    player_id: effect_player_id,
                    ..
                },
            ) => {
                if let Some(query_player_id) = query_player_id {
                    Ok(query_player_id == effect_player_id)
                } else {
                    Ok(true)
                }
            }
            (EffectQuery::DamageDealt { source, target }, Effect::TakeDamage { card_id, .. }) => {
                if let Some(source) = source {
                    if !source.matches(card_id, state) {
                        return Ok(false);
                    }
                }

                if let Some(target) = target {
                    if !target.matches(card_id, state) {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            (EffectQuery::MoveCard { card }, Effect::MoveCard { card_id, .. }) => Ok(card.matches(&card_id, state)),
            (EffectQuery::SummonCard { card }, Effect::SummonCard { card_id, .. }) => Ok(card.matches(card_id, state)),
            (EffectQuery::SummonCard { card }, Effect::SummonCards { cards }) => {
                Ok(cards.into_iter().any(|(_, card_id, _)| card.matches(card_id, state)))
            }
            (EffectQuery::PlayCard { card }, Effect::PlayCard { card_id, .. }) => Ok(card.matches(card_id, state)),
            _ => Ok(false),
        }
    }
}
