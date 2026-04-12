use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct AridDesert {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl AridDesert {
    pub const NAME: &'static str = "Arid Desert";
    pub const DESCRIPTION: &'static str = "Genesis → Deal 1 damage to each minion atop target nearby site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for AridDesert {}

#[async_trait::async_trait]
impl Card for AridDesert {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let picked_site_id = CardQuery::new()
            .sites()
            .near_to(self.get_zone())
            .with_prompt("Red Desert: Pick a site to deal 1 damage to all atop units")
            .pick(&controller_id, state, false)
            .await?;
        if picked_site_id.is_none() {
            return Ok(vec![]);
        }

        let site = state.get_card(&picked_site_id.expect("picked_site_id to not be None"));
        let units = state.get_minions_in_zone(site.get_zone());
        let units = units.iter().filter(|c| c.get_base().region == Region::Surface);
        let mut effects = vec![];
        for unit in units {
            effects.push(Effect::take_damage(&unit.get_id(), site.get_id(), 1));
        }

        Ok(effects)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AridDesert::NAME, |owner_id: PlayerId| {
    Box::new(AridDesert::new(owner_id))
});
