use std::collections::HashMap;

use crate::{
    card::{Ability, AreaModifiers, Card, CardBase, Cost, Edition, Plane, Rarity, Site, SiteBase, SiteType, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct UpdraftRidge {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl UpdraftRidge {
    pub const NAME: &'static str = "Updraft Ridge";

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
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Site for UpdraftRidge {}

#[async_trait::async_trait]
impl Card for UpdraftRidge {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let grants_abilities = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .filter(|c| c.has_modifier(state, &Ability::Airborne))
            .map(|c| (c.get_id().clone(), vec![Ability::Movement(1)]))
            .collect::<HashMap<uuid::Uuid, Vec<Ability>>>();

        AreaModifiers {
            grants_abilities,
            ..Default::default()
        }
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (UpdraftRidge::NAME, |owner_id: PlayerId| {
    Box::new(UpdraftRidge::new(owner_id))
});
