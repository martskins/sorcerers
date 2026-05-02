use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site,
        SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct SmokestacksOfGnaak {
    site_base: SiteBase,
    card_base: CardBase,
}

impl SmokestacksOfGnaak {
    pub const NAME: &'static str = "Smokestacks of Gnaak";
    pub const DESCRIPTION: &'static str = "Other nearby sites lose their abilities.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for SmokestacksOfGnaak {}

#[async_trait::async_trait]
impl Card for SmokestacksOfGnaak {
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

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Disabled,
            affected_cards: CardQuery::new()
                .sites()
                .in_zones(&self.get_zone().get_nearby())
                .id_not_in(vec![*self.get_id()]),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SmokestacksOfGnaak::NAME, |owner_id: PlayerId| {
        Box::new(SmokestacksOfGnaak::new(owner_id))
    });
