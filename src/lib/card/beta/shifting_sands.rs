use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        SiteType, Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct ShiftingSands {
    site_base: SiteBase,
    card_base: CardBase,
}

impl ShiftingSands {
    pub const NAME: &'static str = "Shifting Sands";
    pub const DESCRIPTION: &'static str =
        "Genesis → Reactivate the Genesis abilities of your nearby Deserts.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::Desert],
                tapped: false,
                ..Default::default()
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

impl Site for ShiftingSands {}

#[async_trait::async_trait]
impl Card for ShiftingSands {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let mut effects = vec![];
        for site in CardQuery::new()
            .sites()
            .near_to(self.get_zone())
            .controlled_by(&self.get_controller_id(state))
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ShiftingSands::NAME, |owner_id: PlayerId| {
        Box::new(ShiftingSands::new(owner_id))
    });
