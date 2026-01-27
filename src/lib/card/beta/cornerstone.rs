use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Site, SiteBase, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Cornerstone {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Cornerstone {
    pub const NAME: &'static str = "Cornerstone";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse(""),
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

impl Site for Cornerstone {}

impl Card for Cornerstone {
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

    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        let mut valid_zones = self.default_get_valid_play_zones(state)?;
        let corners = vec![1, 5, 16, 20];
        let valid_corners = corners.iter().filter_map(|c| {
            match state.get_cards_in_zone(&Zone::Realm(*c)).iter().find(|c| c.is_site()) {
                Some(_) => None,
                None => Some(Zone::Realm(*c)),
            }
        });
        valid_zones.extend(valid_corners);
        Ok(valid_zones)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (Cornerstone::NAME, |owner_id: PlayerId| {
    Box::new(Cornerstone::new(owner_id))
});
