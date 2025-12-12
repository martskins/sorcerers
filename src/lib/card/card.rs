use crate::{
    card::{AridDesert, ClamorOfHarpies, Flamecaller},
    effect::Effect,
    game::{PlayerId, PlayerStatus, Thresholds},
    networking::message::ClientMessage,
    state::State,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Spell,
    Avatar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Edition {
    Alpha,
    Beta,
    ArthurianLegends,
    Dragonlord,
    Gothic,
}

impl Edition {
    pub fn url_name(&self) -> &str {
        match self {
            Edition::Alpha => "alp",
            Edition::Beta => "bet",
            Edition::ArthurianLegends => "art",
            Edition::Dragonlord => "drg",
            Edition::Gothic => "got",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Zone {
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Realm(u8),
    Cemetery,
}

pub trait MessageHandler {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub edition: Edition,
    pub zone: Zone,
    pub card_type: CardType,
}

impl CardInfo {
    pub fn is_site(&self) -> bool {
        self.card_type == CardType::Site
    }

    pub fn is_spell(&self) -> bool {
        self.card_type == CardType::Spell
    }
}

pub trait CloneBox {
    fn clone_box(&self) -> Box<dyn Card>;
}

impl<T> CloneBox for T
where
    T: 'static + Card + Clone,
{
    fn clone_box(&self) -> Box<dyn Card> {
        Box::new(self.clone())
    }
}

pub trait Card: Debug + Send + Sync + MessageHandler + CloneBox {
    fn get_name(&self) -> &str;
    fn get_edition(&self) -> Edition;
    fn get_owner_id(&self) -> &PlayerId;
    fn is_tapped(&self) -> bool;
    fn get_card_type(&self) -> CardType;
    fn get_id(&self) -> uuid::Uuid;
    fn get_base_mut(&mut self) -> &mut CardBase;
    fn get_base(&self) -> &CardBase;

    fn get_zone(&self) -> Zone {
        self.get_base().zone.clone()
    }

    fn set_zone(&mut self, zone: Zone) {
        self.get_base_mut().zone = zone;
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }

    fn deathrite(&self, state: &State) -> Vec<Effect> {
        vec![]
    }

    fn is_site(&self) -> bool {
        self.get_card_type() == CardType::Site
    }
}

#[derive(Debug, Clone)]
pub struct SiteBase {
    pub provided_mana: u8,
    pub provided_thresholds: Thresholds,
}

#[derive(Debug, Clone)]
pub struct UnitBase {
    pub power: u8,
    pub toughness: u8,
}

#[derive(Debug, Clone)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub zone: Zone,
}

#[derive(Debug, Clone)]
pub struct AvatarBase {}

pub fn from_name(name: &str, player_id: PlayerId) -> Box<dyn Card> {
    match name {
        Flamecaller::NAME => Box::new(Flamecaller::new(player_id)),
        ClamorOfHarpies::NAME => Box::new(ClamorOfHarpies::new(player_id)),
        AridDesert::NAME => Box::new(AridDesert::new(player_id)),
        _ => panic!("Unknown card name: {}", name),
    }
}

pub fn from_name_and_zone(name: &str, player_id: PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}
