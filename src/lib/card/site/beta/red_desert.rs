use crate::card::{
    site::{site::SiteType, SiteBase},
    CardBase, CardZone, Edition, Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedDesert {
    pub base: SiteBase,
}

impl RedDesert {
    pub const NAME: &'static str = "Red Desert";

    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            base: SiteBase {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    zone,
                    tapped: false,
                    edition: Edition::Beta,
                },
                provided_mana: 1,
                provided_threshold: Thresholds::parse("F"),
                site_types: vec![SiteType::Desert],
            },
        }
    }
}