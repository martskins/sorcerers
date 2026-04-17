use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, SiteType,
        Zone,
    },
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MountainPass {
    site_base: SiteBase,
    card_base: CardBase,
}

impl MountainPass {
    pub const NAME: &'static str = "Mountain Pass";
    pub const DESCRIPTION: &'static str =
        "Minions can't enter this site on the ground if there's already a minion atop.";

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

impl Site for MountainPass {
    fn can_be_entered_by(
        &self,
        card_id: &uuid::Uuid,
        _from: &Zone,
        region: &Region,
        state: &State,
    ) -> bool {
        let minions_atop = self
            .get_zone()
            .get_minions(state, None)
            .iter()
            .filter(|c| c.get_region(state) == &Region::Surface)
            .count();

        let card = state.get_card(card_id);
        let ground_movement =
            card.get_region(state) == &Region::Surface && region == &Region::Surface;
        !ground_movement || minions_atop == 0
    }
}

#[async_trait::async_trait]
impl Card for MountainPass {
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (MountainPass::NAME, |owner_id: PlayerId| {
    Box::new(MountainPass::new(owner_id))
});
