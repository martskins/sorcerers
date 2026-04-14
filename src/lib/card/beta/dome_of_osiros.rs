use crate::{
    card::{
        Ability, Card, CardBase, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct DomeOfOsiros {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl DomeOfOsiros {
    pub const NAME: &'static str = "Dome of Osiros";
    pub const DESCRIPTION: &'static str = "This site and minions here can't be attacked.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::new(),
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
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for DomeOfOsiros {}

#[async_trait::async_trait]
impl Card for DomeOfOsiros {
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

        Ok(vec![
            // The site itself cannot be attacked.
            ContinuousEffect::GrantAbility {
                ability: Ability::Unattackable,
                affected_cards: CardQuery::from_id(self.get_id().clone()),
            },
            // Minions here cannot be attacked.
            ContinuousEffect::GrantAbility {
                ability: Ability::Unattackable,
                affected_cards: CardQuery::new().in_zone(self.get_zone()).minions(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DomeOfOsiros::NAME, |owner_id: PlayerId| {
        Box::new(DomeOfOsiros::new(owner_id))
    });
