use crate::prelude::*;

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
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for VantageHills {}

impl ResourceProvider for VantageHills {}

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

    fn area_modifiers(&self, _state: &State) -> Vec<OngoingEffect> {
        vec![OngoingEffect::GrantAbility {
            ability: Ability::Ranged(1),
            affected_cards: CardQuery::new().units().in_zone_of_card(self.get_id()),
        }]
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (VantageHills::NAME, |owner_id: PlayerId| {
    Box::new(VantageHills::new(owner_id))
});
