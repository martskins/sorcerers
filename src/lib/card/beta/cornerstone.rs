use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Cornerstone {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Cornerstone {
    pub const NAME: &'static str = "Cornerstone";
    pub const DESCRIPTION: &'static str = "You may play this site to any corner.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse(""),
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

#[async_trait::async_trait]
impl Site for Cornerstone {}

impl ResourceProvider for Cornerstone {}

impl Card for Cornerstone {
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

    fn get_valid_play_locations(
        &self,
        state: &State,
        player_id: &PlayerId,
        caster_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<Location>> {
        let mut valid_zones = self.base_get_valid_play_locations(state, player_id, caster_id)?;
        let corners: [Location; 4] = [
            Location::Square(1, Region::Surface),
            Location::Square(5, Region::Surface),
            Location::Square(16, Region::Surface),
            Location::Square(20, Region::Surface),
        ];
        let valid_corners: Vec<Location> = corners
            .into_iter()
            .filter(|location| location.get_site(state).is_none())
            .collect();
        valid_zones.extend(valid_corners);
        Ok(valid_zones)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Cornerstone::NAME, |owner_id: PlayerId| {
    Box::new(Cornerstone::new(owner_id))
});
