
use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct VantageHills {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl VantageHills {
    pub const NAME: &'static str = "Vantage Hills";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for VantageHills {}

#[async_trait::async_trait]
impl Card for VantageHills {
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

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (VantageHills::NAME, |owner_id: PlayerId| {
    Box::new(VantageHills::new(owner_id))
});
