use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{yes_or_no, PlayerId},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

/// **Queen of Midland** — Unique Minion (5 cost, 1/2)
///
/// After an opponent draws a card, if they have more cards than you, you may draw a card.
#[derive(Debug, Clone)]
pub struct QueenOfMidland {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl QueenOfMidland {
    pub const NAME: &'static str = "Queen of Midland";
    pub const DESCRIPTION: &'static str =
        "After an opponent draws a card, if they have more cards than you, you may draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "EE"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for QueenOfMidland {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }
    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let queen_id = *self.get_id();
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::DrawCard {
                    player_id: Some(opponent_id),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(queen_id),
                }),
                on_effect: Arc::new(move |state: &State, _: &uuid::Uuid, _: &Effect| {
                    Box::pin(async move {
                        let my_hand = CardQuery::new()
                            .in_zone(&Zone::Hand)
                            .controlled_by(&controller_id)
                            .all(state)
                            .len();
                        let opp_hand = CardQuery::new()
                            .in_zone(&Zone::Hand)
                            .controlled_by(&opponent_id)
                            .all(state)
                            .len();
                        if opp_hand <= my_hand {
                            return Ok(vec![]);
                        }
                        let draw =
                            yes_or_no(&controller_id, state, "Queen of Midland: Draw a card?")
                                .await?;
                        if draw {
                            Ok(vec![Effect::DrawCard {
                                player_id: controller_id,
                                count: 1,
                            }])
                        } else {
                            Ok(vec![])
                        }
                    })
                        as Pin<
                            Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                        >
                }),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (QueenOfMidland::NAME, |owner_id: PlayerId| {
        Box::new(QueenOfMidland::new(owner_id))
    });
