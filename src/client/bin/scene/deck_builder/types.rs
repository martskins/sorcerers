use sorcerers::{
    card::{CardData, CardType, Edition, Rarity, Region},
    game::{PlayerId, Thresholds},
    zone::Zone,
};

#[derive(Clone, PartialEq)]
pub(super) enum ElemFilter {
    All,
    Fire,
    Air,
    Earth,
    Water,
}

#[derive(Clone, PartialEq)]
pub(super) enum TypeFilter {
    All,
    Minion,
    Site,
    Spell,
}

#[derive(Clone)]
pub struct CardEntry {
    pub name: String,
    pub card_type: CardType,
    pub zone: Zone,
    pub rarity: Rarity,
    pub mana: u8,
    pub thresholds: Thresholds,
    pub image_path: String,
    pub power: Option<u16>,
    pub toughness: Option<u16>,
}

impl CardEntry {
    pub fn max_copies(&self) -> u8 {
        match self.rarity {
            Rarity::Ordinary => 4,
            Rarity::Exceptional => 3,
            Rarity::Elite => 2,
            Rarity::Unique => 1,
        }
    }

    pub(super) fn as_card_data(&self) -> CardData {
        CardData {
            id: uuid::Uuid::nil(),
            name: self.name.clone(),
            owner_id: PlayerId::nil(),
            controller_id: PlayerId::nil(),
            tapped: false,
            edition: Edition::Beta,
            zone: Zone::Spellbook,
            region: Region::Surface,
            card_type: self.card_type.clone(),
            abilities: vec![],
            damage_taken: 0,
            bearer: None,
            rarity: self.rarity.clone(),
            power: self.power.unwrap_or(0),
            has_attachments: false,
            image_path: self.image_path.clone(),
            is_token: false,
        }
    }
}
