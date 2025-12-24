use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, SiteBase, Zone},
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
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
            },
        }
    }
}

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

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_valid_play_zones(&self, state: &State) -> Vec<Zone> {
        let mut valid_zones = self.default_get_valid_play_zones(state);
        let corners = vec![1, 5, 16, 20];
        let valid_corners = corners.iter().filter_map(|c| {
            match state.get_cards_in_zone(&Zone::Realm(*c)).iter().find(|c| c.is_site()) {
                Some(_) => None,
                None => Some(Zone::Realm(*c)),
            }
        });
        valid_zones.extend(valid_corners);
        valid_zones
    }
}
