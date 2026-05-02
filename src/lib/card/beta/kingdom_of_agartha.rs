use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site,
        SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct KingdomOfAgartha {
    site_base: SiteBase,
    card_base: CardBase,
}

impl KingdomOfAgartha {
    pub const NAME: &'static str = "Kingdom of Agartha";
    pub const DESCRIPTION: &'static str = "(E)(E)(E) — All minions have Burrowing.";

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

impl Site for KingdomOfAgartha {}

#[async_trait::async_trait]
impl Card for KingdomOfAgartha {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let thresholds = state.get_thresholds_for_player(&controller_id);
        if thresholds.earth < 3 {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Burrowing,
            affected_cards: CardQuery::new().minions(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (KingdomOfAgartha::NAME, |owner_id: PlayerId| {
        Box::new(KingdomOfAgartha::new(owner_id))
    });
