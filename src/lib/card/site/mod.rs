use crate::{
    card::{CardBase, CardZone},
    effect::Effect,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Site {
    Beacon(CardBase),
    Bog(CardBase),
    AnnualFair(CardBase),
}

impl Site {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Site::Beacon(cb) => &cb.id,
            Site::Bog(cb) => &cb.id,
            Site::AnnualFair(cb) => &cb.id,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Site::Beacon(_) => "Beacon",
            Site::Bog(_) => "Bog",
            Site::AnnualFair(_) => "Annual Fair",
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Site::Beacon(cb) => &cb.owner_id,
            Site::Bog(cb) => &cb.owner_id,
            Site::AnnualFair(cb) => &cb.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Site::Beacon(cb) => &cb.zone,
            Site::Bog(cb) => &cb.zone,
            Site::AnnualFair(cb) => &cb.zone,
        }
    }

    pub fn set_zone(&mut self, new_zone: CardZone) {
        match self {
            Site::Beacon(cb) => cb.zone = new_zone,
            Site::Bog(cb) => cb.zone = new_zone,
            Site::AnnualFair(cb) => cb.zone = new_zone,
        };
    }

    pub fn genesis(&self) -> Vec<Effect> {
        match self {
            _ => {
                vec![Effect::AddMana {
                    player_id: self.get_owner_id().clone(),
                    amount: 1,
                }]
            }
        }
    }

    pub fn on_turn_start(&self) -> Vec<Effect> {
        match self {
            _ => {
                vec![Effect::AddMana {
                    player_id: self.get_owner_id().clone(),
                    amount: 1,
                }]
            }
        }
    }
}
