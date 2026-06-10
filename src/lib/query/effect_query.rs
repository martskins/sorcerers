use crate::{
    card::{Ability, Region},
    effect::{DrawKind, Effect},
    game::PlayerId,
    query::{CardQuery, ZoneQuery},
    state::State,
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
        ability: Ability,
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
    Fight {
        attacker: CardQuery,
        defender: Option<CardQuery>,
    },
}

impl EffectQuery {
    pub async fn matches(&self, effect: &Effect, state: &State) -> anyhow::Result<bool> {
        match (self, effect) {
            (
                EffectQuery::Fight {
                    attacker: attacker_query,
                    defender: defender_query,
                },
                Effect::Fight {
                    attacker_id,
                    defender_id,
                    ..
                },
            ) => {
                let attacker_matches = attacker_query.matches(attacker_id, state);
                let defender_matches = defender_query
                    .as_ref()
                    .is_none_or(|dq| dq.matches(defender_id, state));
                Ok(attacker_matches && defender_matches)
            }
            (
                EffectQuery::EnterZone {
                    card: card_query,
                    zone: zone_query,
                    ..
                },
                Effect::SummonCards { summoned_cards },
            ) => {
                for sc in summoned_cards {
                    let card_matches = card_query.matches(&sc.card_id, state);
                    let zone_matches =
                        zone_query.matches(state, &sc.to_location.clone().into_zone());
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
                    card_id, location, ..
                },
            ) => {
                if from.is_some() {
                    return Ok(false);
                }

                let card_matches = card_query.matches(card_id, state);
                let zone_matches = zone_query.matches(state, &location.clone().into_zone());
                if card_matches && zone_matches {
                    return Ok(true);
                }

                Ok(false)
            }
            (
                EffectQuery::StopAtZone {
                    card: card_query,
                    zone: zone_query,
                },
                Effect::MoveCard {
                    player_id,
                    card_id,
                    to,
                    ..
                },
            ) => {
                let dest = to.pick(player_id, state).await?.into_zone();
                let zone_matches = zone_query.matches(state, &dest);
                let card_matches = card_query.matches(card_id, state);
                Ok(zone_matches && card_matches)
            }
            (
                EffectQuery::TurnStart {
                    player_id: query_player_id,
                },
                Effect::StartTurn {
                    player_id: effect_player_id,
                    ..
                },
            ) => Ok(query_player_id.is_none_or(|p| p == *effect_player_id)),
            (
                EffectQuery::TurnEnd {
                    player_id: query_player_id,
                },
                Effect::EndTurn {
                    player_id: effect_player_id,
                    ..
                },
            ) => Ok(query_player_id.is_none_or(|p| p == *effect_player_id)),
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
                    println!("Source doesn't match");
                    return Ok(false);
                }

                if let Some(target) = target
                    && !target.matches(card_id, state)
                {
                    println!("Target doesn't match");
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
            (EffectQuery::SummonCard { card }, Effect::SummonCards { summoned_cards }) => {
                Ok(summoned_cards
                    .iter()
                    .any(|sc| card.matches(&sc.card_id, state)))
            }
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
            ) => Ok(query_pid.is_none_or(|p| p == *player_id)),
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
