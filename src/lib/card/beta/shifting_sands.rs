use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct ShiftingSands {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl ShiftingSands {
    pub const NAME: &'static str = "Shifting Sands";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::Desert],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

impl Site for ShiftingSands {}

#[async_trait::async_trait]
impl Card for ShiftingSands {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let mut effects = vec![];
        for site in CardMatcher::sites_near(self.get_zone())
            .controller_id(&self.get_controller_id(state))
            .site_types(vec![SiteType::Desert])
            .iter(state)
        {
            effects.extend(site.genesis(state).await?);
        }
        Ok(effects)
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ShiftingSands::NAME, |owner_id: PlayerId| {
    Box::new(ShiftingSands::new(owner_id))
});
