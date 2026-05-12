use crate::{
    card::Region,
    effect::Effect,
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
    pub async fn matches(&self, effect: &Effect, state: &State) -> anyhow::Result<bool> {
        match (self, effect) {
            (
                EffectQuery::EnterZone { card, zone },
                Effect::MoveCard {
                    player_id,
                    card_id,
                    to,
                    ..
                },
            ) => {
                let zones = zone.options(state);
                let zone = to.pick(player_id, state).await?;
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
            (EffectQuery::UntapCard { card }, Effect::UntapCard { card_id }) => {
                Ok(card.matches(card_id, state))
            }
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
            (EffectQuery::MoveCard { card }, Effect::MoveCard { card_id, .. }) => {
                Ok(card.matches(card_id, state))
            }
            (EffectQuery::SummonCard { card }, Effect::SummonCard { card_id, .. }) => {
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
                Effect::DrawSpell { player_id, .. },
            ) => Ok(optional_player_matches(query_pid, player_id)),
            (
                EffectQuery::DrawCard {
                    player_id: query_pid,
                },
                Effect::DrawSite { player_id, .. },
            ) => Ok(optional_player_matches(query_pid, player_id)),
            (
                EffectQuery::DrawCard {
                    player_id: query_pid,
                },
                Effect::DrawCard { player_id, .. },
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

fn optional_player_matches(query: &Option<PlayerId>, actual: &PlayerId) -> bool {
    query.as_ref().is_none_or(|q| q == actual)
}
