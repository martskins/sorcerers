use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase,
        Zone,
    },
    game::{PlayerId, Thresholds},
    state::State,
};

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

impl Site for Cornerstone {}

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

    fn get_valid_play_zones(
        &self,
        state: &State,
        player_id: &PlayerId,
    ) -> anyhow::Result<Vec<Zone>> {
        let mut valid_zones = self.default_get_valid_play_zones(state, player_id)?;
        let corners: [Zone; 4] = [
            Zone::Realm(1),
            Zone::Realm(5),
            Zone::Realm(16),
            Zone::Realm(20),
        ];
        let valid_corners: Vec<Zone> = corners
            .into_iter()
            .filter(|z| z.get_site(state).is_none())
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
