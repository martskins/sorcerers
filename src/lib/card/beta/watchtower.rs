use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, Rarity,
        ResourceProvider, Site, SiteBase, SiteType, Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Watchtower {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Watchtower {
    pub const NAME: &'static str = "Watchtower";
    pub const DESCRIPTION: &'static str =
        "Enemy units at nearby sites lose Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("A"),
                types: vec![SiteType::Tower],
                tapped: false,
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

impl Site for Watchtower {}

#[async_trait::async_trait]
impl Card for Watchtower {
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
        let controller_id = self.get_controller_id(state);
        let nearby_zones = self.get_zone().get_nearby();

        let enemy_stealth_units: std::collections::HashMap<uuid::Uuid, Vec<crate::card::Ability>> =
            CardQuery::new()
                .units()
                .in_zones(&nearby_zones)
                .all(state)
                .into_iter()
                .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
                .map(|id| (id, vec![Ability::Stealth]))
                .collect();

        AreaModifiers {
            removes_abilities: enemy_stealth_units,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Watchtower::NAME, |owner_id: PlayerId| {
    Box::new(Watchtower::new(owner_id))
});
