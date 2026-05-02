use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity,
        Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct RestInPeace {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl RestInPeace {
    pub const NAME: &'static str = "Rest in Peace";
    pub const DESCRIPTION: &'static str =
        "Spirits and Undead summoned to affected sites are burrowed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "EE"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Aura for RestInPeace {}

#[async_trait::async_trait]
impl Card for RestInPeace {
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
                trigger_on_effect: EffectQuery::SummonCard {
                    card: CardQuery::new()
                        .minions()
                        .minion_types(vec![MinionType::Spirit, MinionType::Undead]),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(aura_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, minion_id: &uuid::Uuid, _effect: &Effect| {
                        let minion_id = *minion_id;
                        Box::pin(async move {
                            if !state.get_card(&aura_id).get_zone().is_in_play() {
                                return Ok(vec![]);
                            }
                            let affected_zones =
                                if let Some(aura) = state.get_card(&aura_id).get_aura() {
                                    aura.get_affected_zones(state)
                                } else {
                                    return Ok(vec![]);
                                };
                            let minion_zone = state.get_card(&minion_id).get_zone().clone();
                            if !affected_zones.contains(&minion_zone) {
                                return Ok(vec![]);
                            }
                            Ok(vec![Effect::SetCardRegion {
                                card_id: minion_id,
                                region: Region::Underground,
                                tap: false,
                            }])
                        })
                            as Pin<
                                Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                            >
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RestInPeace::NAME, |owner_id: PlayerId| {
    Box::new(RestInPeace::new(owner_id))
});
