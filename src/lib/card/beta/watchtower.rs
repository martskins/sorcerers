use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Watchtower {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Watchtower {
    pub const NAME: &'static str = "Watchtower";
    pub const DESCRIPTION: &'static str = "Enemy units atop nearby sites permanently lose Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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
impl Site for Watchtower {}

impl ResourceProvider for Watchtower {}

#[async_trait::async_trait]
impl Card for Watchtower {
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

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::RemoveAbilities {
            removal: AbilityRemoval::Exact(vec![Ability::Stealth]),
            affected_cards: Box::new(CardQuery::new()
                .units()
                .not_controlled_by(&self.get_controller_id(state))
                .near_to(&self.get_location().with_region(Region::Surface))
                .with_ability(Ability::Stealth)),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Watchtower::NAME, |owner_id: PlayerId| {
    Box::new(Watchtower::new(owner_id))
});
