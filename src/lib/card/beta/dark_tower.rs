use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct DarkTower {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl DarkTower {
    pub const NAME: &'static str = "Dark Tower";

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
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for DarkTower {}

#[async_trait::async_trait]
impl Card for DarkTower {
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
            .filter(|c| c.get_zone().is_in_realm())
            .filter(|c| c.get_id() != self.get_id())
            .filter(|c| c.get_controller_id() == self.get_owner_id())
            .filter(|c| c.get_name() == Self::NAME)
            .count();
        if count > 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::AddResources {
            player_id: self.get_owner_id().clone(),
            mana: 1,
            thresholds: Thresholds::new(),
            health: 0,
        }])
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}
