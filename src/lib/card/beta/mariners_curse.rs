use std::{future::Future, pin::Pin, sync::Arc};

use crate::{prelude::*, query::entered_sites};

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
            aura_base: AuraBase { tapped: false },
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

    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let aura_id = *self.get_id();

        let affected_zones = self.get_affected_zones(state);
        let affected_water_sites = CardQuery::new()
            .water_sites()
            .in_zones(&affected_zones)
            .all(state);
        let zones = affected_water_sites
            .into_iter()
            .map(|site_id| state.get_card(&site_id).get_zone().clone())
            .collect::<Vec<_>>();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::EnterSite {
                    card: CardQuery::new().minions(),
                    site: ZoneQuery::from_options(zones, None),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: aura_id.into(),
                }),
                multitrigger: false,
                on_effect: Arc::new(move |state: &State, card_id: &CardId, effect: &Effect| {
                    Box::pin(async move {
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

                        let entered_affected_water = entered_sites(effect, state)
                            .await?
                            .into_iter()
                            .filter(|(entered_card_id, _)| entered_card_id == card_id)
                            .map(|(_, site_zone)| site_zone)
                            .any(|site_zone| {
                                affected_zones.contains(&site_zone)
                                    && site_zone
                                        .get_site(state)
                                        .and_then(|site| site.is_water_site(state).ok())
                                        .unwrap_or(false)
                            });

                        if !entered_affected_water {
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
                        as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MarinersCurse::NAME, |owner_id: PlayerId| {
        Box::new(MarinersCurse::new(owner_id))
    });
