use crate::{
    card::Region,
    effect::{DrawKind, Effect},
    game::{CardId, PlayerId},
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
        from: Option<ZoneQuery>,
    },
    StopAtZone {
        card: CardQuery,
        zone: ZoneQuery,
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
        spellcaster: Option<CardQuery>,
    },
    LifeLost {
        player_id: PlayerId,
        from_attack: Option<bool>,
    },
    SummonCard {
        card: CardQuery,
    },
    Genesis {
        card: CardQuery,
    },
    Deathrite {
        card: CardQuery,
    },
    BuryCard {
        card: CardQuery,
    },
    SetCardRegion {
        card: CardQuery,
        destination: Option<Region>,
    },
    RangedStrike {
        striker: CardQuery,
    },
    Attack {
        attacker: CardQuery,
        defender: Option<CardQuery>,
    },
    DefendDeclared {
        attacker: CardQuery,
        defender: CardQuery,
    },
    DrawCard {
        player_id: Option<PlayerId>,
    },
    UnitKilled {
        unit: CardQuery,
        killer: Option<CardQuery>,
        from_attack: Option<bool>,
    },
    StrikeCard {
        card: CardQuery,
        striker: Option<CardQuery>,
    },
}

impl EffectQuery {
    pub async fn source_ids(&self, effect: &Effect, state: &State) -> anyhow::Result<Vec<CardId>> {
        match (self, effect) {
            // TODO: Implement this
            // (
            //     EffectQuery::LifeLost {
            //         player_id: target_player_id,
            //         from_attack,
            //     },
            //     Effect::AdjustAvatarLife { player_id, amount },
            // ) => Ok(vec![]),
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
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    ..
                },
                Effect::SummonCards { cards },
            ) => {
                for (_, card, _, loc) in cards {
                    let card_matches = card_query.matches(card, state);
                    let zone_matches = zone_query.matches(state, &loc.clone().into_zone());
                    if card_matches && zone_matches {
                        return Ok(vec![*card]);
                    }
                }

                Ok(vec![])
            }
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    ..
                },
                Effect::MoveCard {
                    player_id,
                    card_id,
                    to,
                    ..
                },
            ) => {
                let card_matches = card_query.matches(card_id, state);
                let loc = to.pick(player_id, state).await?;
                let zone_matches = zone_query.matches(state, &loc.into_zone());
                if card_matches && zone_matches {
                    return Ok(vec![*card_id]);
                }

                Ok(vec![])
            }
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    from,
                },
                Effect::PlayCard {
                    player_id,
                    card_id,
                    zone,
                    ..
                },
            ) => {
                if from.is_some() {
                    return Ok(vec![]);
                }

                let card_matches = card_query.matches(card_id, state);
                let picked_zone = zone.pick(player_id, state).await?;
                let zone_matches = zone_query.matches(state, &picked_zone);
                if card_matches && zone_matches {
                    return Ok(vec![*card_id]);
                }

                Ok(vec![])
            }
            (EffectQuery::StopAtZone { card, zone: site }, _) => {
                let sites = site.options(state);
                Ok(stopped_at_sites(effect, state)
                    .await?
                    .into_iter()
                    .filter(|(card_id, stopped_site)| {
                        card.matches(card_id, state) && sites.contains(stopped_site)
                    })
                    .map(|(card_id, _)| card_id)
                    .collect())
            }
            (EffectQuery::SummonCard { card }, Effect::SummonCards { cards }) => Ok(cards
                .iter()
                .filter(|(_, card_id, _, _)| card.matches(card_id, state))
                .map(|(_, card_id, _, _)| *card_id)
                .collect()),
            (EffectQuery::Genesis { card }, Effect::TriggerGenesis { card_id }) => {
                if card.matches(card_id, state) {
                    Ok(vec![*card_id])
                } else {
                    Ok(vec![])
                }
            }
            (EffectQuery::Deathrite { card }, Effect::TriggerDeathrite { card_id, .. }) => {
                if card.matches(card_id, state) {
                    Ok(vec![*card_id])
                } else {
                    Ok(vec![])
                }
            }
            (
                EffectQuery::UnitKilled {
                    unit,
                    killer,
                    from_attack: from_attack_target,
                },
                Effect::KillMinion {
                    card_id,
                    killer_id,
                    from_attack,
                    ..
                },
            ) => {
                let card_matches = unit.matches(card_id, state);
                let killer_matches = killer.clone().is_none_or(|k| k.matches(killer_id, state));
                let from_attack_matches = from_attack_target.is_none_or(|fa| fa == *from_attack);
                if card_matches && killer_matches && from_attack_matches {
                    Ok(vec![*card_id])
                } else {
                    Ok(vec![])
                }
            }
            (
                EffectQuery::StrikeCard { card, striker },
                Effect::TakeDamage {
                    card_id,
                    from,
                    damage,
                },
            ) => {
                let card_matches = card.matches(card_id, state);
                let striker_matches = striker.clone().is_none_or(|k| k.matches(from, state));
                let is_strike = damage.is_strike;
                if is_strike && card_matches && striker_matches {
                    Ok(vec![*card_id])
                } else {
                    Ok(vec![])
                }
            }
            (
                EffectQuery::RangedStrike { striker },
                Effect::ShootProjectile {
                    shooter,
                    ranged_strike,
                    ..
                },
            ) => {
                let striker_matches = striker.matches(shooter, state);
                if *ranged_strike && striker_matches {
                    return Ok(vec![*shooter]);
                }

                Ok(vec![])
            }
            (
                EffectQuery::DefendDeclared { attacker, defender },
                Effect::DeclareDefender {
                    attacker_id,
                    defender_id,
                },
            ) => {
                if attacker.matches(attacker_id, state) && defender.matches(defender_id, state) {
                    Ok(vec![*defender_id])
                } else {
                    Ok(vec![])
                }
            }
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
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    ..
                },
                Effect::SummonCards { cards },
            ) => {
                for (_, card, _, loc) in cards {
                    let card_matches = card_query.matches(card, state);
                    let zone_matches = zone_query.matches(state, &loc.clone().into_zone());
                    if card_matches && zone_matches {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    ..
                },
                Effect::MoveCard {
                    player_id,
                    card_id,
                    to,
                    ..
                },
            ) => {
                let card_matches = card_query.matches(card_id, state);
                let loc = to.pick(player_id, state).await?;
                let zone_matches = zone_query.matches(state, &loc.into_zone());
                if card_matches && zone_matches {
                    return Ok(true);
                }

                Ok(false)
            }
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    from,
                },
                Effect::PlayCard {
                    player_id,
                    card_id,
                    zone,
                    ..
                },
            ) => {
                if from.is_some() {
                    return Ok(false);
                }

                let card_matches = card_query.matches(card_id, state);
                let picked_zone = zone.pick(player_id, state).await?;
                let zone_matches = zone_query.matches(state, &picked_zone);
                if card_matches && zone_matches {
                    return Ok(true);
                }

                Ok(false)
            }
            (EffectQuery::StopAtZone { .. }, _) => {
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
                .any(|(_, card_id, _, _)| card.matches(card_id, state))),
            (EffectQuery::Genesis { card }, Effect::TriggerGenesis { card_id }) => {
                Ok(card.matches(card_id, state))
            }
            (EffectQuery::Deathrite { card }, Effect::TriggerDeathrite { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (
                EffectQuery::PlayCard { card, spellcaster },
                Effect::PlayMagic {
                    card_id, caster_id, ..
                },
            ) => {
                let spellcaster_matches = spellcaster
                    .as_ref()
                    .is_none_or(|sp| sp.matches(caster_id, state));
                Ok(spellcaster_matches && card.matches(card_id, state))
            }
            (
                EffectQuery::PlayCard {
                    card,
                    spellcaster: target_spellcaster,
                },
                Effect::PlayCard {
                    card_id,
                    spellcaster: actual_spellcaster,
                    ..
                },
            ) => {
                let spellcaster_matches = target_spellcaster
                    .as_ref()
                    .is_none_or(|sp| sp.matches(actual_spellcaster, state));
                Ok(spellcaster_matches && card.matches(card_id, state))
            }
            (EffectQuery::BuryCard { card }, Effect::BuryCard { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (
                EffectQuery::SetCardRegion {
                    card,
                    destination: target_dest,
                },
                Effect::SetCardRegion {
                    card_id,
                    destination: actual_dest,
                    ..
                },
            ) => {
                let card_matches = card.matches(card_id, state);
                let dest_matches = target_dest.as_ref().is_none_or(|r| r == actual_dest);
                Ok(card_matches && dest_matches)
            }
            (
                EffectQuery::Attack { attacker, defender },
                Effect::DeclareAttack {
                    attacker_id,
                    target_id,
                },
            ) => {
                if !attacker.matches(attacker_id, state) {
                    return Ok(false);
                }

                if let Some(defender) = defender
                    && !defender.matches(target_id, state)
                {
                    return Ok(false);
                }

                Ok(true)
            }
            (
                EffectQuery::DefendDeclared { attacker, defender },
                Effect::DeclareDefender {
                    attacker_id,
                    defender_id,
                },
            ) => Ok(attacker.matches(attacker_id, state) && defender.matches(defender_id, state)),
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

            (
                EffectQuery::UnitKilled {
                    unit,
                    killer,
                    from_attack: from_attack_target,
                },
                Effect::KillMinion {
                    card_id,
                    killer_id,
                    from_attack,
                    ..
                },
            ) => {
                let card_matches = unit.matches(card_id, state);
                let killer_matches = killer.clone().is_none_or(|k| k.matches(killer_id, state));
                let from_attack_matches = from_attack_target.is_none_or(|fa| fa == *from_attack);
                Ok(card_matches && killer_matches && from_attack_matches)
            }
            (
                EffectQuery::StrikeCard { card, striker },
                Effect::TakeDamage {
                    card_id,
                    from,
                    damage,
                },
            ) => {
                let card_matches = card.matches(card_id, state);
                let striker_matches = striker.clone().is_none_or(|k| k.matches(from, state));
                let is_strike = damage.is_strike;
                Ok(is_strike && card_matches && striker_matches)
            }
            (
                EffectQuery::RangedStrike { striker },
                Effect::ShootProjectile {
                    shooter,
                    ranged_strike,
                    ..
                },
            ) => {
                let striker_matches = striker.matches(shooter, state);
                Ok(*ranged_strike && striker_matches)
            }
            _ => Ok(false),
        }
    }
}

pub async fn entered_zones(
    effect: &Effect,
    state: &State,
) -> anyhow::Result<Vec<(uuid::Uuid, Zone, Zone)>> {
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
                    entered.push((*card_id, previous_zone.clone(), zone.clone()));
                }
                previous_zone = zone;
            }

            Ok(entered)
        }
        Effect::SummonCards { cards } => Ok(cards
            .iter()
            .map(|(_, card_id, from_zone, location)| {
                (*card_id, from_zone.clone(), location.clone().into_zone())
            })
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
            .filter_map(|(_, card_id, _, location)| {
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

pub async fn stopped_at_sites(
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
            let final_zone = match through_path {
                Some(path) => path.last().cloned(),
                None => Some(to.pick(player_id, state).await?.into_zone()),
            };
            let Some(final_zone) = final_zone else {
                return Ok(vec![]);
            };
            if from.clone().into_zone() == final_zone {
                return Ok(vec![]);
            }
            let Some(site) = final_zone.get_site_at_square(state) else {
                return Ok(vec![]);
            };

            let stopped_cards = std::iter::once(*card_id)
                .chain(CardQuery::new().carried_by(card_id).all(state))
                .map(|stopped_id| (stopped_id, site.get_zone().clone()))
                .collect();
            Ok(stopped_cards)
        }
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
