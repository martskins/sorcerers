use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Site, SiteBase, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct AstralAlcazar {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl AstralAlcazar {
    pub const NAME: &'static str = "Astral Alcazar";
    pub const DESCRIPTION: &'static str = "Units can move between this site and any void as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for AstralAlcazar {}

#[async_trait::async_trait]
impl Card for AstralAlcazar {
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

    fn area_modifiers(&self, state: &State) -> crate::card::AreaModifiers {
        // TODO: This ability is not quite right. It should grant the ability of moving to any void
        // as if it were adjacent to this site.
        let grants_abilities = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .map(|unit| (unit.get_id().clone(), vec![Ability::Voidwalk]))
            .collect();

        crate::card::AreaModifiers {
            grants_abilities,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AstralAlcazar::NAME, |owner_id: PlayerId| {
    Box::new(AstralAlcazar::new(owner_id))
});
