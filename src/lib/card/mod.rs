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
    Realm,
    Spellbook,
    Atlasbook,
    DiscardPile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: uuid::Uuid,
    pub image: String,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
    pub name: String,
    pub card_type: CardType,
    pub mana_cost: u32,
    pub description: Option<String>,
    pub tapped: bool,
}
