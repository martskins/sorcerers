use std::collections::HashMap;

use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, Rarity,
        ResourceProvider, Site, SiteBase, SiteType, Zone,
    },
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RiverOfFlame {
    site_base: SiteBase,
    card_base: CardBase,
}

impl RiverOfFlame {
    pub const NAME: &'static str = "River of Flame";
    pub const DESCRIPTION: &'static str = "Fire Spellcaster";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("F"),
                types: vec![SiteType::River],
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

impl Site for RiverOfFlame {}

#[async_trait::async_trait]
impl Card for RiverOfFlame {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        if !self.get_zone().is_in_play() {
            return AreaModifiers::default();
        }
        let grants: HashMap<uuid::Uuid, Vec<Ability>> = state
            .cards
            .iter()
            .filter(|c| c.get_unit_base().is_some())
            .filter(|c| c.get_zone() == self.get_zone())
            .map(|c| (*c.get_id(), vec![Ability::Spellcaster(Some(Element::Fire))]))
            .collect();
        AreaModifiers {
            grants_abilities: grants,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RiverOfFlame::NAME, |owner_id: PlayerId| {
    Box::new(RiverOfFlame::new(owner_id))
});
