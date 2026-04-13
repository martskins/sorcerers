use crate::{
    card::{
        Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct DwarvenForge {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl DwarvenForge {
    pub const NAME: &'static str = "Dwarven Forge";
    pub const DESCRIPTION: &'static str =
        "Anyone may conjure Weapons and Armor here, and for ① less.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for DwarvenForge {}

#[async_trait::async_trait]
impl Card for DwarvenForge {
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

    // TODO: Implementation is missing the effect of letting anyone conjure Weapons and Armor here
    // for ① less.
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DwarvenForge::NAME, |owner_id: PlayerId| {
        Box::new(DwarvenForge::new(owner_id))
    });
