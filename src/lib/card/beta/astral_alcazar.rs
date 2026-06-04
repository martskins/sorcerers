use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct AstralAlcazar {
    site_base: SiteBase,
    card_base: CardBase,
}

impl AstralAlcazar {
    pub const NAME: &'static str = "Astral Alcazar";
    pub const DESCRIPTION: &'static str =
        "Units can move between this site and any void as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
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
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for AstralAlcazar {}

impl ResourceProvider for AstralAlcazar {}

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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let connected_zones = Zone::all_in_region(Region::Void)
            .into_iter()
            .filter(|zone| zone.get_site_at_square(state).is_none())
            .collect();

        Ok(vec![OngoingEffect::ConnectZones {
            connected_zones,
            affected_cards: CardQuery::new().units().in_zone_of_card(self.get_id()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AstralAlcazar::NAME, |owner_id: PlayerId| {
        Box::new(AstralAlcazar::new(owner_id))
    });
