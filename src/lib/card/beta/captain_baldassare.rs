use std::sync::Arc;

use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct CaptainBaldassare {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl CaptainBaldassare {
    pub const NAME: &'static str = "Captain Baldassare";
    pub const DESCRIPTION: &'static str = "Whenever Captain Baldassare attacks a unit or site, the defending player discards their topmost three spells. You may cast each of those spells once this turn, ignoring threshold requirements.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CaptainBaldassare {
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

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::Attack {
                    attacker: CardQuery::from_id(self.get_id().clone()),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self.get_id().clone()),
                }),
                on_effect: Arc::new(
                    move |state: &State, _card_id: &uuid::Uuid, effect: &Effect| {
                        Box::pin(async move {
                            let Effect::Attack { defender_id, .. } = effect else {
                                return Ok(vec![]);
                            };
                            let defender = state.get_card(defender_id);
                            let defending_player = defender.get_controller_id(state);

                            // Discard top 3 spells from the defending player's deck.
                            let deck = state.decks.get(&defending_player).ok_or_else(|| {
                                anyhow::anyhow!("No deck for player {:?}", defending_player)
                            })?;
                            let top_three: Vec<uuid::Uuid> =
                                deck.spells.iter().rev().take(3).cloned().collect();

                            let effects: Vec<Effect> = top_three
                                .iter()
                                .map(|spell_id| Effect::DiscardCard {
                                    player_id: defending_player.clone(),
                                    card_id: spell_id.clone(),
                                })
                                .collect();

                            // TODO: Allow casting those spells ignoring threshold.
                            // This requires framework support for "cast ignoring threshold" flag.

                            Ok(effects)
                        })
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CaptainBaldassare::NAME, |owner_id: PlayerId| {
        Box::new(CaptainBaldassare::new(owner_id))
    });
