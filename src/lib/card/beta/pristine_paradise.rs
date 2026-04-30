use crate::{
    card::{
        CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
};

/// **Pristine Paradise** — Unique Site (all thresholds AEFW)
///
/// Provides no mana or threshold unless completely empty.
/// TODO: Implement "provides nothing unless completely empty" mechanic.
#[derive(Debug, Clone)]
pub struct PristineParadise {
    site_base: SiteBase,
    card_base: CardBase,
}

impl PristineParadise {
    pub const NAME: &'static str = "Pristine Paradise";
    pub const DESCRIPTION: &'static str = "Provides no mana or threshold unless completely empty.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("AEFW"),
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
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for PristineParadise {}

#[async_trait::async_trait]
impl crate::card::Card for PristineParadise {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PristineParadise::NAME, |owner_id: PlayerId| {
        Box::new(PristineParadise::new(owner_id))
    });
