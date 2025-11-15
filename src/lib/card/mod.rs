pub mod avatar;
pub mod site;
pub mod spell;

use crate::{
    card::{avatar::Avatar, site::Site, spell::Spell},
    effect::Effect,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Spell,
    Avatar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardZone {
    Hand,
    Spellbook,
    Atlasbook,
    DiscardPile,
    Avatar,
    Realm(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Card {
    Site(Site),
    Spell(Spell),
    Avatar(Avatar),
}

impl Card {
    pub fn is_site(&self) -> bool {
        matches!(self, Card::Site(_))
    }

    pub fn is_avatar(&self) -> bool {
        matches!(self, Card::Avatar(_))
    }

    pub fn is_spell(&self) -> bool {
        matches!(self, Card::Spell(_))
    }

    pub fn get_image(&self) -> String {
        let name = match self {
            Card::Site(site) => site.get_name(),
            Card::Spell(spell) => spell.get_name(),
            Card::Avatar(avatar) => avatar.get_name(),
        };
        format!("assets/images/cards/{}.png", name).to_string()
    }

    pub fn get_type(&self) -> CardType {
        match self {
            Card::Site(_) => CardType::Site,
            Card::Spell(_) => CardType::Spell,
            Card::Avatar(_) => CardType::Avatar,
        }
    }

    pub fn set_zone(&mut self, zone: CardZone) {
        match self {
            Card::Site(card) => card.set_zone(zone),
            Card::Spell(card) => card.set_zone(zone),
            Card::Avatar(card) => card.set_zone(zone),
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Card::Site(card) => &card.get_owner_id(),
            Card::Spell(card) => &card.get_owner_id(),
            Card::Avatar(card) => &card.get_owner_id(),
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Card::Site(card) => &card.get_zone(),
            Card::Spell(card) => &card.get_zone(),
            Card::Avatar(card) => &card.get_zone(),
        }
    }

    pub fn get_card_type(&self) -> CardType {
        match self {
            Card::Site(_) => CardType::Site,
            Card::Spell(_) => CardType::Spell,
            Card::Avatar(_) => CardType::Avatar,
        }
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Card::Site(card) => &card.get_id(),
            Card::Spell(card) => &card.get_id(),
            Card::Avatar(card) => &card.get_id(),
        }
    }

    pub fn genesis(&self) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.genesis(),
            Card::Site(card) => card.genesis(),
            Card::Avatar(_card) => vec![],
        }
    }
}
