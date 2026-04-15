use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider,
        Site, SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct VantageHills {
    site_base: SiteBase,
    card_base: CardBase,
}

impl VantageHills {
    pub const NAME: &'static str = "Vantage Hills";
    pub const DESCRIPTION: &'static str = "Ranged units atop this site have +1 range.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("E"),
                types: vec![],
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
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let grants_abilities = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .filter(|c| c.get_region(state) == &Region::Surface)
            .map(|c| (c.get_id().clone(), vec![Ability::Ranged(1)]))
            .collect();

        AreaModifiers {
            grants_abilities,
            ..Default::default()
        }
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (VantageHills::NAME, |owner_id: PlayerId| {
        Box::new(VantageHills::new(owner_id))
    });
