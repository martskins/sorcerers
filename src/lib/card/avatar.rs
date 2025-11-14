use crate::card::{CardBase, CardZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Avatar {
    Sorcerer(CardBase),
    Battlemage(CardBase),
}

impl Avatar {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Avatar::Sorcerer(cb) => &cb.id,
            Avatar::Battlemage(cb) => &cb.id,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Avatar::Sorcerer(_) => "Sorcerer",
            Avatar::Battlemage(_) => "Battlemage",
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Avatar::Sorcerer(cb) => &cb.owner_id,
            Avatar::Battlemage(cb) => &cb.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Avatar::Sorcerer(cb) => &cb.zone,
            Avatar::Battlemage(cb) => &cb.zone,
        }
    }

    pub fn set_zone(&mut self, zone: CardZone) {
        match self {
            Avatar::Sorcerer(cb) => cb.zone = zone,
            Avatar::Battlemage(cb) => cb.zone = zone,
        };
    }
}
