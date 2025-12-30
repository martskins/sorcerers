use rand::seq::IndexedRandom;

use crate::{
    card::Zone,
    effect::Effect,
    game::{PlayerId, pick_card, pick_card_with_preview, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub enum ZoneQuery {
    Any {
        prompt: Option<String>,
    },
    AnySite {
        controlled_by: Option<PlayerId>,
        prompt: Option<String>,
    },
    Specific(Zone),
    Random {
        options: Vec<Zone>,
    },
    FromOptions {
        options: Vec<Zone>,
        prompt: Option<String>,
    },
}

impl ZoneQuery {
    pub fn prompt(&self) -> &str {
        match self {
            ZoneQuery::Any { prompt } => {
                if let Some(p) = prompt {
                    p.as_ref()
                } else {
                    "Pick a zone"
                }
            }
            ZoneQuery::Specific(_) => "Pick a zone",
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
            ZoneQuery::Specific(z) => vec![z.clone()],
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
            ZoneQuery::Random { options } => {
                vec![options.choose(&mut rand::rng()).unwrap().clone()]
            }
            ZoneQuery::FromOptions { options, .. } => options.clone(),
        }
    }
    pub async fn resolve(&self, player_id: &PlayerId, state: &State) -> Zone {
        match self {
            ZoneQuery::Any { prompt } => {
                pick_zone(
                    player_id,
                    &Zone::all_realm(),
                    state,
                    prompt.as_ref().map_or("Pick a zone", |v| v),
                )
                .await
            }
            ZoneQuery::Specific(z) => z.clone(),
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
            ZoneQuery::Random { options } => {
                for card in &state.cards {
                    if let Some(query) = card.zone_query_override(state, self) {
                        return Box::pin(query.resolve(player_id, state)).await;
                    }
                }

                options.choose(&mut rand::rng()).unwrap().clone()
            }
            ZoneQuery::FromOptions { options, prompt } => {
                pick_zone(player_id, options, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CardQuery {
    Specific(uuid::Uuid),
    InZone {
        zone: Zone,
        owner: Option<PlayerId>,
        prompt: Option<String>,
    },
    NearZone {
        zone: Zone,
        owner: Option<PlayerId>,
        prompt: Option<String>,
    },
    OwnedBy {
        owner: uuid::Uuid,
        prompt: Option<String>,
    },
    RandomUnitInZone {
        zone: Zone,
    },
    FromOptions {
        options: Vec<uuid::Uuid>,
        prompt: Option<String>,
        preview: bool,
    },
}

impl CardQuery {
    pub fn options(&self, state: &State) -> Vec<uuid::Uuid> {
        match self {
            CardQuery::Specific(id) => vec![id.clone()],
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
        }
    }

    pub async fn resolve(&self, player_id: &PlayerId, state: &State) -> uuid::Uuid {
        match self {
            CardQuery::Specific(id) => id.clone(),
            CardQuery::InZone { zone, owner, prompt } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_units(state, owner.as_ref())
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
            CardQuery::NearZone { zone, owner, prompt } => {
                let cards: Vec<uuid::Uuid> = zone
                    .get_nearby_units(state, owner.as_ref())
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
            CardQuery::OwnedBy { owner, prompt } => {
                let cards: Vec<uuid::Uuid> = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == owner)
                    .map(|c| c.get_id().clone())
                    .collect();
                pick_card(player_id, &cards, state, prompt.as_ref().map_or("Pick a zone", |v| v)).await
            }
            CardQuery::RandomUnitInZone { zone } => {
                for card in &state.cards {
                    if let Some(query) = card.card_query_override(state, self) {
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

                pick_card(player_id, options, state, prompt.as_ref().unwrap_or(&String::new())).await
            }
        }
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
