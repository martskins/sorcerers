use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Site, SiteBase, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct Rubble {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Rubble {
    pub const NAME: &'static str = "Rubble";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::parse(""),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Token,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for Rubble {}

impl Card for Rubble {
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
}
