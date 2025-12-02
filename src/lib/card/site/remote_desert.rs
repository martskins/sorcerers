use crate::{
    card::{site::SiteBase, CardBase, CardZone, Edition},
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDesert {
    pub base: SiteBase,
}

impl RemoteDesert {
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
            },
        }
    }
}