use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct EdgeOfTheWorld {
    site_base: SiteBase,
    card_base: CardBase,
}

impl EdgeOfTheWorld {
    pub const NAME: &'static str = "Edge of the World";
    pub const DESCRIPTION: &'static str = "Must always be adjacent to the void.";

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
impl Site for EdgeOfTheWorld {}

impl ResourceProvider for EdgeOfTheWorld {}

#[async_trait::async_trait]
impl Card for EdgeOfTheWorld {
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

    fn get_valid_play_locations(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        Ok(self
            .base_get_valid_play_locations(state, player_id, caster_id)?
            .into_iter()
            .filter(|location| !location.get_adjacent_voids(state).is_empty())
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (EdgeOfTheWorld::NAME, |owner_id: PlayerId| {
        Box::new(EdgeOfTheWorld::new(owner_id))
    });
