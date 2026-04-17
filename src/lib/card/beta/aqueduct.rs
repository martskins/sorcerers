use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct Aqueduct {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Aqueduct {
    pub const NAME: &'static str = "Aqueduct";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("EW"),
                types: vec![],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Aqueduct {}

#[async_trait::async_trait]
impl Card for Aqueduct {
    fn get_name(&self) -> &str {
        Self::NAME
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (Aqueduct::NAME, |owner_id: PlayerId| {
    Box::new(Aqueduct::new(owner_id))
});
