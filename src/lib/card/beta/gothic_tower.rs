use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct GothicTower {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl GothicTower {
    pub const NAME: &'static str = "Gothic Tower";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for GothicTower {}

#[async_trait::async_trait]
impl Card for GothicTower {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let count = state
            .cards
            .iter()
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_id() != self.get_id())
            .filter(|c| &c.get_controller_id(state) == self.get_owner_id())
            .filter(|c| c.get_name() == Self::NAME)
            .count();
        if count > 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::AddResources {
            player_id: self.get_owner_id().clone(),
            mana: 1,
        }])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (GothicTower::NAME, |owner_id: PlayerId| {
    Box::new(GothicTower::new(owner_id))
});
