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
        cache.game_queries.entry(game_id).or_default().push(matcher_id);
    }

    pub async fn matcher_results(matcher_id: &uuid::Uuid) -> Option<Vec<uuid::Uuid>> {
        let cache = QUERY_CACHE.get().expect("to get lock").read().await;
        cache.matcher_queries.get(matcher_id).cloned()
    }

    pub async fn store_effect_targets(game_id: uuid::Uuid, effect_id: uuid::Uuid, affected_cards: Vec<uuid::Uuid>) {
        let mut cache = QUERY_CACHE.get().expect("to get lock").write().await;
        cache.effect_targets.insert(effect_id.clone(), affected_cards);

        cache.game_queries.entry(game_id).or_default().push(effect_id.clone());
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
            .get(&qry.id)
        {
            return Ok(cached.clone());
        }

        let zone = if let Some(zone) = &qry.zone {
            zone.clone()
        } else if qry.random {
            let options = qry.options.as_deref().unwrap_or(&[]);
            for card in &state.cards {
                if let Some(query) = card.zone_query_override(state, qry)? {
                    return Ok(Box::pin(query.resolve(player_id, state)).await?);
                }
            }
            options
                .choose(&mut rand::rng())
                .expect("failed to get random zone")
                .clone()
        } else if let Some(options) = &qry.options {
            pick_zone(player_id, options, state, false, qry.prompt()).await?
        } else if qry.sites_only {
            let mut sites: Vec<Zone> = state
                .cards
                .iter()
                .filter(|c| c.is_site())
                .filter(|c| c.get_zone().is_in_play())
                .filter(|c| {
                    qry.controlled_by
                        .as_ref()
                        .map_or(true, |p| c.get_controller_id(state) == *p)
                })
                .map(|c| c.get_zone().clone())
                .collect();
            sites.dedup();
            pick_zone(player_id, &sites, state, false, qry.prompt()).await?
        } else {
            pick_zone(player_id, &Zone::all_realm(), state, false, qry.prompt()).await?
        };

        let mut cache = QUERY_CACHE.get().expect("failed to get random zone").write().await;
        cache.zone_queries.insert(qry.id, zone.clone());
        cache.game_queries.entry(state.game_id).or_default().push(qry.id);

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
pub struct ZoneQuery {
    id: uuid::Uuid,
    /// A fixed zone — resolves immediately without prompting the player.
    zone: Option<Zone>,
    /// Explicit list of zones to pick from (or randomly select from when `random` is true).
    options: Option<Vec<Zone>>,
    /// When true, a zone is chosen randomly from `options` (subject to `zone_query_override`).
    random: bool,
    /// When true, the option pool is restricted to in-play site zones.
    sites_only: bool,
    /// Optionally filter `sites_only` results to zones controlled by this player.
    controlled_by: Option<PlayerId>,
    prompt: Option<String>,
}

impl Default for ZoneQuery {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            zone: None,
            options: None,
            random: false,
            sites_only: false,
            controlled_by: None,
            prompt: None,
        }
    }
}

impl From<Zone> for ZoneQuery {
    fn from(zone: Zone) -> Self {
        ZoneQuery::from_zone(zone)
    }
}

impl From<&Zone> for ZoneQuery {
    fn from(zone: &Zone) -> Self {
        ZoneQuery::from_zone(zone.clone())
    }
}

impl ZoneQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_randomised(&self) -> bool {
        self.random
    }

    /// Resolves to a specific zone without prompting.
    pub fn from_zone(zone: Zone) -> Self {
        Self {
            zone: Some(zone),
            ..Self::default()
        }
    }

    pub fn adjacent_to(self, zone: &Zone) -> Self {
        let zones = zone.get_adjacent();
        Self {
            options: Some(zones),
            ..self
        }
    }

    pub fn near(self, zone: &Zone) -> Self {
        let zones = zone.get_nearby();
        Self {
            options: Some(zones),
            ..self
        }
    }

    /// Player picks from in-play site zones, optionally filtered by controller.
    pub fn any_site(controlled_by: Option<PlayerId>, prompt: Option<String>) -> Self {
        Self {
            sites_only: true,
            controlled_by,
            prompt,
            ..Self::default()
        }
    }

    /// A zone is chosen randomly from `options` (cards may override via `zone_query_override`).
    pub fn random(options: Vec<Zone>) -> Self {
        Self {
            options: Some(options),
            random: true,
            ..Self::default()
        }
    }

    /// Player picks from the given list of zones.
    pub fn from_options(options: Vec<Zone>, prompt: Option<String>) -> Self {
        Self {
            options: Some(options),
            prompt,
            ..Self::default()
        }
    }

    pub fn with_prompt(self, prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            ..self
        }
    }

    pub fn randomised(self) -> Self {
        Self { random: true, ..self }
    }

    fn prompt(&self) -> &str {
        self.prompt.as_deref().unwrap_or(if self.sites_only {
            "Pick a site zone"
        } else {
            "Pick a zone"
        })
    }

    /// Returns the set of candidate zones for this query given current game state.
    pub fn options(&self, state: &State) -> Vec<Zone> {
        if let Some(zone) = &self.zone {
            return vec![zone.clone()];
        }
        if self.random {
            let opts = self.options.as_deref().unwrap_or(&[]);
            return vec![
                opts.choose(&mut rand::rng())
                    .expect("failed to get random zone")
                    .clone(),
            ];
        }
        if let Some(opts) = &self.options {
            return opts.clone();
        }
        if self.sites_only {
            let mut sites: Vec<Zone> = state
                .cards
                .iter()
                .filter(|c| c.is_site())
                .filter(|c| c.get_zone().is_in_play())
                .filter(|c| {
                    self.controlled_by
                        .as_ref()
                        .map_or(true, |p| c.get_controller_id(state) == *p)
                })
                .map(|c| c.get_zone().clone())
                .collect();
            sites.dedup();
            return sites;
        }
        Zone::all_realm()
    }

    /// Resolves the query, prompting the player if needed. Caches the result.
    pub async fn pick(&self, player_id: &PlayerId, state: &State) -> anyhow::Result<Zone> {
        QueryCache::resolve_zone(self, player_id, state).await
    }

    /// Alias for `pick` — use `pick` for new call sites.
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

fn optional_player_matches(query: &Option<PlayerId>, actual: &PlayerId) -> bool {
    query.as_ref().map_or(true, |q| q == actual)
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
            ) => Ok(optional_player_matches(query_player_id, effect_player_id)),
            (
                EffectQuery::TurnEnd {
                    player_id: query_player_id,
                },
                Effect::EndTurn {
                    player_id: effect_player_id,
                    ..
                },
            ) => Ok(optional_player_matches(query_player_id, effect_player_id)),
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
