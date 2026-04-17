use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    query::{EffectQuery, ZoneQuery},
    state::{CardQuery, DeferredEffect, State},
};
#[derive(Debug, Clone)]
pub struct BottomlessPit {
    site_base: SiteBase,
    card_base: CardBase,
}

impl BottomlessPit {
    pub const NAME: &'static str = "Bottomless Pit";
    pub const DESCRIPTION: &'static str =
        "Whenever a non-Airborne minion enters this site, kill it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn kill_non_airborne_trigger(&self) -> DeferredEffect {
        let my_zone = self.get_zone().clone();
        DeferredEffect {
            trigger_on_effect: EffectQuery::EnterZone {
                card: CardQuery::new().minions(),
                zone: ZoneQuery::from_zone(my_zone),
            },
            expires_on_effect: Some(EffectQuery::BuryCard {
                card: self.get_id().into(),
            }),
            on_effect: Arc::new(
                move |state: &State, card_id: &uuid::Uuid, _effect: &Effect| {
                    let card_id = *card_id;
                    Box::pin(async move {
                        let card = state.get_card(&card_id);
                        if card.has_ability(state, &Ability::Airborne) {
                            return Ok(vec![]);
                        }
                        Ok(vec![Effect::BuryCard { card_id }])
                    })
                        as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                },
            ),
            multitrigger: true,
        }
    }
}

impl Site for BottomlessPit {}

#[async_trait::async_trait]
impl Card for BottomlessPit {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::AddDeferredEffect {
            effect: self.kill_non_airborne_trigger(),
        }])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BottomlessPit::NAME, |owner_id: PlayerId| {
        Box::new(BottomlessPit::new(owner_id))
    });
