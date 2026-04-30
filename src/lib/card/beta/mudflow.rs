use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, ResourceProvider, Site,
        SiteBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Mudflow {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Mudflow {
    pub const NAME: &'static str = "Mudflow";
    pub const DESCRIPTION: &'static str =
        "At the start of your turn, you may surface or unburrow all minions at a nearby site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
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

impl Site for Mudflow {}

#[async_trait::async_trait]
impl Card for Mudflow {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let Some(target_zone_id) = CardQuery::new()
            .sites()
            .near_to(self.get_zone())
            .with_prompt("Mudflow: Pick a nearby site to surface/unburrow all minions")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target_site = state.get_card(&target_zone_id);
        let target_zone = target_site.get_zone().clone();

        let minions = CardQuery::new()
            .minions()
            .in_zone(&target_zone)
            .all(state);

        let effects = minions
            .into_iter()
            .map(|minion_id| Effect::SetCardRegion {
                card_id: minion_id,
                region: Region::Surface,
                tap: false,
            })
            .collect();

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Mudflow::NAME, |owner_id: PlayerId| {
    Box::new(Mudflow::new(owner_id))
});
