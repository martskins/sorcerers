use crate::{
    card::Region,
    effect::{DrawKind, Effect},
    game::PlayerId,
    query::{CardQuery, ZoneQuery},
    state::State,
    zone::Zone,
};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum EffectQuery {
    OneOf(Vec<EffectQuery>),
    EnterZone {
        card: CardQuery,
        zone: ZoneQuery,
    },
    EnterSite {
        card: CardQuery,
        site: ZoneQuery,
    },
    DamageDealt {
        source: Option<CardQuery>,
        target: Option<CardQuery>,
    },
    RemoveAbility {
        card: CardQuery,
        ability: crate::card::Ability,
    },
    TurnEnd {
        player_id: Option<PlayerId>,
    },
    TurnStart {
        player_id: Option<PlayerId>,
    },
    UntapCard {
        card: CardQuery,
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
    BuryCard {
        card: CardQuery,
    },
    SetCardRegion {
        card: CardQuery,
        region: Option<Region>,
    },
    Attack {
        attacker: CardQuery,
    },
    DrawCard {
        player_id: Option<PlayerId>,
    },
}

impl EffectQuery {
    pub async fn source_ids(
        &self,
        effect: &Effect,
        state: &State,
    ) -> anyhow::Result<Vec<uuid::Uuid>> {
        match (self, effect) {
            (EffectQuery::OneOf(queries), _) => {
                let mut source_ids = vec![];
                for query in queries {
                    for source_id in Box::pin(query.source_ids(effect, state)).await? {
                        if !source_ids.contains(&source_id) {
                            source_ids.push(source_id);
                        }
                    }
                }
                Ok(source_ids)
            }
            (EffectQuery::EnterZone { card, zone }, _) => {
                let zones = zone.options(state);
                Ok(entered_zones(effect, state)
                    .await?
                    .into_iter()
                    .filter(|(card_id, entered_zone)| {
                        card.matches(card_id, state) && zones.contains(entered_zone)
                    })
                    .map(|(card_id, _)| card_id)
                    .collect())
            }
            (EffectQuery::EnterSite { card, site }, _) => {
                let sites = site.options(state);
                Ok(entered_sites(effect, state)
                    .await?
                    .into_iter()
                    .filter(|(card_id, entered_site)| {
                        card.matches(card_id, state) && sites.contains(entered_site)
                    })
                    .map(|(card_id, _)| card_id)
                    .collect())
            }
            (EffectQuery::SummonCard { card }, Effect::SummonCards { cards }) => Ok(cards
                .iter()
                .filter(|(_, card_id, _)| card.matches(card_id, state))
                .map(|(_, card_id, _)| *card_id)
                .collect()),
            (_, _) => {
                if Box::pin(self.matches(effect, state)).await? {
                    Ok(effect
                        .source_id()
                        .map(|source_id| vec![*source_id])
                        .unwrap_or_default())
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    pub async fn matches(&self, effect: &Effect, state: &State) -> anyhow::Result<bool> {
        match (self, effect) {
            (EffectQuery::EnterZone { .. }, _) => {
                Ok(!self.source_ids(effect, state).await?.is_empty())
            }
            (EffectQuery::EnterSite { .. }, _) => {
                Ok(!self.source_ids(effect, state).await?.is_empty())
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
            (
                EffectQuery::UntapCard { card },
                Effect::SetTapped {
                    card_id,
                    tapped: false,
                },
            ) => Ok(card.matches(card_id, state)),
            (
                EffectQuery::DamageDealt { source, target },
                Effect::TakeDamage { card_id, from, .. },
            ) => {
                if let Some(source) = source
                    && !source.matches(from, state)
                {
                    return Ok(false);
                }

                if let Some(target) = target
                    && !target.matches(card_id, state)
                {
                    return Ok(false);
                }

                Ok(true)
            }
            (
                EffectQuery::RemoveAbility { card, ability },
                Effect::RemoveAbility { card_id, modifier },
            ) => Ok(card.matches(card_id, state) && ability == modifier),
            (EffectQuery::MoveCard { card }, Effect::MoveCard { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (EffectQuery::SummonCard { card }, Effect::SummonCards { cards }) => Ok(cards
                .iter()
                .any(|(_, card_id, _)| card.matches(card_id, state))),
            (EffectQuery::PlayCard { card }, Effect::PlayMagic { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (EffectQuery::PlayCard { card }, Effect::PlayCard { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (EffectQuery::BuryCard { card }, Effect::BuryCard { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (
                EffectQuery::SetCardRegion { card, region },
                Effect::SetCardRegion {
                    card_id,
                    region: effect_region,
                    ..
                },
            ) => {
                Ok(card.matches(card_id, state)
                    && region.as_ref().is_none_or(|r| r == effect_region))
            }
            (EffectQuery::Attack { attacker }, Effect::Attack { attacker_id, .. }) => {
                Ok(attacker.matches(attacker_id, state))
            }
            (
                EffectQuery::DrawCard {
                    player_id: query_pid,
                },
                Effect::DrawCard {
                    player_id,
                    kind: DrawKind::Spell | DrawKind::Site | DrawKind::Choice,
                    ..
                },
            ) => Ok(optional_player_matches(query_pid, player_id)),
            (EffectQuery::OneOf(queries), effect) => {
                for query in queries {
                    if Box::pin(query.matches(effect, state)).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

pub async fn entered_zones(
    effect: &Effect,
    state: &State,
) -> anyhow::Result<Vec<(uuid::Uuid, Zone)>> {
    match effect {
        Effect::MoveCard {
            player_id,
            card_id,
            from,
            to,
            through_path,
            ..
        } => {
            let mut entered = vec![];
            let mut previous_zone = from.clone().into_zone();
            let zones = match through_path {
                Some(path) => path.clone(),
                None => vec![to.pick(player_id, state).await?.into_zone()],
            };

            for zone in zones {
                if previous_zone != zone {
                    entered.push((*card_id, zone.clone()));
                }
                previous_zone = zone;
            }

            Ok(entered)
        }
        Effect::SummonCards { cards } => Ok(cards
            .iter()
            .map(|(_, card_id, location)| (*card_id, location.clone().into_zone()))
            .collect()),
        _ => Ok(vec![]),
    }
}

pub async fn entered_sites(
    effect: &Effect,
    state: &State,
) -> anyhow::Result<Vec<(uuid::Uuid, Zone)>> {
    match effect {
        Effect::MoveCard {
            player_id,
            card_id,
            from,
            to,
            through_path,
            ..
        } => {
            let mut entered = vec![];
            let mut previous_zone = from.clone().into_zone();
            let zones = match through_path {
                Some(path) => path.clone(),
                None => vec![to.pick(player_id, state).await?.into_zone()],
            };

            for zone in zones {
                if let Some(site_zone) = entered_site(&previous_zone, &zone, state) {
                    entered.push((*card_id, site_zone));
                }
                previous_zone = zone;
            }

            Ok(entered)
        }
        Effect::SummonCards { cards } => Ok(cards
            .iter()
            .filter_map(|(_, card_id, location)| {
                location
                    .clone()
                    .into_zone()
                    .get_site_at_square(state)
                    .map(|site| (*card_id, site.get_zone().clone()))
            })
            .collect()),
        _ => Ok(vec![]),
    }
}

pub fn entered_site(from: &Zone, to: &Zone, state: &State) -> Option<Zone> {
    let to_site = to.get_site_at_square(state)?;
    let to_site_zone = to_site.get_zone();
    let from_site_zone = from.get_site_at_square(state).map(|site| site.get_zone());

    if from_site_zone == Some(to_site_zone) {
        None
    } else {
        Some(to_site_zone.clone())
    }
}

fn optional_player_matches(query: &Option<PlayerId>, actual: &PlayerId) -> bool {
    query.as_ref().is_none_or(|q| q == actual)
}
