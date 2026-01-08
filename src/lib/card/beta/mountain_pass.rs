use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MountainPass {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl MountainPass {
    pub const NAME: &'static str = "Mountain Pass";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
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

impl Site for MountainPass {
    fn can_be_entered_by(&self, card_id: &uuid::Uuid, _from: &Zone, plane: &Plane, state: &State) -> bool {
        let minions_atop = self
            .get_zone()
            .get_minions(state, None)
            .iter()
            .filter(|c| c.get_base().plane == Plane::Surface)
            .count();

        let card = state.get_card(card_id);
        let ground_movement = card.get_plane(state) == &Plane::Surface && plane == &Plane::Surface;
        !ground_movement || minions_atop == 0
    }
}

#[async_trait::async_trait]
impl Card for MountainPass {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (MountainPass::NAME, |owner_id: PlayerId| {
    Box::new(MountainPass::new(owner_id))
});
