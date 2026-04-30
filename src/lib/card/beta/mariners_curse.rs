use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct MarinersCurse {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl MarinersCurse {
    pub const NAME: &'static str = "Mariner's Curse";
    pub const DESCRIPTION: &'static str = "When a minion enters an affected water site, it submerges. Then return Mariner's Curse to your hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for MarinersCurse {}

#[async_trait::async_trait]
impl Card for MarinersCurse {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let aura_id = *self.get_id();

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::MoveCard {
                    card: CardQuery::new().minions(),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: aura_id.into(),
                }),
                multitrigger: false,
                on_effect: Arc::new(
                    move |state: &State, _minion_id: &uuid::Uuid, effect: &Effect| {
                        Box::pin(async move {
                            let Effect::MoveCard {
                                card_id,
                                to,
                                player_id,
                                ..
                            } = effect
                            else {
                                return Ok(vec![]);
                            };

                            let to_zone = to.resolve(player_id, state).await?;

                            // Check if aura is still in play.
                            if !state.get_card(&aura_id).get_zone().is_in_play() {
                                return Ok(vec![]);
                            }

                            // Get the aura's affected zones.
                            let aura = state.get_card(&aura_id);
                            let affected_zones = if let Some(a) = aura.get_aura() {
                                a.get_affected_zones(state)
                            } else {
                                return Ok(vec![]);
                            };

                            if !affected_zones.contains(&to_zone) {
                                return Ok(vec![]);
                            }

                            // Check if destination is a water site.
                            let is_water = to_zone
                                .get_site(state)
                                .and_then(|s| s.is_water_site(state).ok())
                                .unwrap_or(false);
                            if !is_water {
                                return Ok(vec![]);
                            }

                            let _aura_owner = aura.get_owner_id();
                            Ok(vec![
                                Effect::SetCardRegion {
                                    card_id: *card_id,
                                    region: Region::Underwater,
                                    tap: false,
                                },
                                Effect::SetCardZone {
                                    card_id: aura_id,
                                    zone: Zone::Hand,
                                },
                            ])
                        })
                            as Pin<
                                Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                            >
                    },
                ),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MarinersCurse::NAME, |owner_id: PlayerId| {
        Box::new(MarinersCurse::new(owner_id))
    });
