use crate::{
    card::{CardBase, CardZone},
    effect::Effect,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spell {
    BurningHands(CardBase),
    BallLightning(CardBase),
}

impl Spell {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Spell::BurningHands(cb) => &cb.id,
            Spell::BallLightning(cb) => &cb.id,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Spell::BurningHands(_) => "Burning Hands",
            Spell::BallLightning(_) => "Ball Lightning",
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Spell::BurningHands(cb) => &cb.owner_id,
            Spell::BallLightning(cb) => &cb.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Spell::BurningHands(cb) => &cb.zone,
            Spell::BallLightning(cb) => &cb.zone,
        }
    }

    pub fn set_zone(&mut self, new_zone: CardZone) {
        match self {
            Spell::BurningHands(cb) => cb.zone = new_zone,
            Spell::BallLightning(cb) => cb.zone = new_zone,
        };
    }

    pub fn genesis(&self) -> Vec<Effect> {
        vec![]
        // Implement site-specific on_cast effects here
    }
}
