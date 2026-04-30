use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, Rarity,
        ResourceProvider, Site, SiteBase, Zone,
    },
    game::{PlayerId, Thresholds},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Iceberg {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Iceberg {
    pub const NAME: &'static str = "Iceberg";
    pub const DESCRIPTION: &'static str = "Minions nearby cannot Submerge or surface.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
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

impl Site for Iceberg {}

#[async_trait::async_trait]
impl Card for Iceberg {
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

        let nearby_minions = CardQuery::new()
            .minions()
            .near_to(self.get_zone())
            .all(state);

        let removes = nearby_minions
            .iter()
            .map(|id| (*id, vec![Ability::Submerge]))
            .collect();

        AreaModifiers {
            removes_abilities: removes,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Iceberg::NAME, |owner_id: PlayerId| {
    Box::new(Iceberg::new(owner_id))
});
