use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Spell,
    Avatar,
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
    pub fn get_image(&self) -> String {
        let name = match self {
            Card::Site(site) => &site.name,
            Card::Spell(spell) => &spell.name,
            Card::Avatar(avatar) => &avatar.name,
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
            Card::Site(card) => card.zone = zone,
            Card::Spell(card) => card.zone = zone,
            Card::Avatar(card) => card.zone = zone,
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Card::Site(card) => &card.owner_id,
            Card::Spell(card) => &card.owner_id,
            Card::Avatar(card) => &card.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Card::Site(card) => &card.zone,
            Card::Spell(card) => &card.zone,
            Card::Avatar(card) => &card.zone,
        }
    }

    pub fn get_card_type(&self) -> CardType {
        match self {
            Card::Site(_) => CardType::Site,
            Card::Spell(_) => CardType::Spell,
            Card::Avatar(_) => CardType::Avatar,
        }
    }

    pub fn get_id(&self) -> uuid::Uuid {
        match self {
            Card::Site(card) => card.id,
            Card::Spell(card) => card.id,
            Card::Avatar(card) => card.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Avatar {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spell {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
    pub card_type: CardType,
    pub mana_cost: u32,
    pub description: Option<String>,
    pub tapped: bool,
}
