use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct TheGeistwood {
    site_base: SiteBase,
    card_base: CardBase,
}

impl TheGeistwood {
    pub const NAME: &'static str = "The Geistwood";
    pub const DESCRIPTION: &'static str = "Genesis abilities here are also Deathrite abilities.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
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

#[async_trait::async_trait]
impl Site for TheGeistwood {
    // TODO: Deathrite is not modeled as an ability trigger yet.
}

impl ResourceProvider for TheGeistwood {}

#[async_trait::async_trait]
impl Card for TheGeistwood {
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (TheGeistwood::NAME, |owner_id: PlayerId| {
    Box::new(TheGeistwood::new(owner_id))
});
