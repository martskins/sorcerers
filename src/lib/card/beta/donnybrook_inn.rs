use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone},
    game::{PlayerId, Thresholds},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct DonnybrookInn {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl DonnybrookInn {
    pub const NAME: &'static str = "Donnybrook Inn";
    pub const DESCRIPTION: &'static str =
        "Anyone may cast minions here and they may do so for ① less.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for DonnybrookInn {}

#[async_trait::async_trait]
impl Card for DonnybrookInn {
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

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![
            ContinuousEffect::ModifyManaCost {
                mana_diff: -1,
                affected_cards: CardQuery::new().minions(),
                zones: Some(vec![self.get_zone().clone()]),
            },
            ContinuousEffect::OverrideValidPlayZone {
                affected_zones: vec![self.get_zone().clone()],
                affected_cards: CardQuery::new().minions().in_zone(&Zone::Hand),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DonnybrookInn::NAME, |owner_id: PlayerId| {
        Box::new(DonnybrookInn::new(owner_id))
    });
