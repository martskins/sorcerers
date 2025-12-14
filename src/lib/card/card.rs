use crate::{
    card::{beta, AridDesert, ClamorOfHarpies, Flamecaller, PitVipers},
    effect::Effect,
    game::{are_adjacent, are_nearby, Element, PlayerId, Thresholds},
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
    fn get_id(&self) -> &uuid::Uuid;
    fn get_base(&self) -> &CardBase;
    fn get_base_mut(&mut self) -> &mut CardBase;

    fn has_modifier(&self, state: &State, ability: Modifier) -> bool {
        match self.get_unit_base() {
            Some(ub) => ub.abilities.contains(&ability),
            None => false,
        }
    }

    fn get_elements(&self, state: &State) -> Vec<Element> {
        let mut elements = Vec::new();
        let thresholds = self.get_required_thresholds(state);
        if thresholds.fire > 0 {
            elements.push(Element::Fire);
        }
        if thresholds.water > 0 {
            elements.push(Element::Water);
        }
        if thresholds.earth > 0 {
            elements.push(Element::Earth);
        }
        if thresholds.air > 0 {
            elements.push(Element::Air);
        }
        elements
    }

    fn get_square(&self) -> Option<u8> {
        match self.get_zone() {
            Zone::Realm(sq) => Some(sq),
            _ => None,
        }
    }

    fn get_valid_move_squares(&self, state: &State) -> Vec<u8> {
        let square = match self.get_zone() {
            Zone::Realm(sq) => sq,
            _ => unreachable!(),
        };

        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_site())
            .filter_map(|c| match c.get_zone() {
                Zone::Realm(s) => {
                    if self.has_modifier(state, Modifier::Airborne) && are_nearby(s, square) || are_adjacent(s, square)
                    {
                        Some(s)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    fn get_valid_attack_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        let square = match self.get_zone() {
            Zone::Realm(sq) => sq,
            _ => unreachable!(),
        };

        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() != self.get_owner_id())
            .filter(|c| c.is_unit() || c.is_site())
            .filter_map(|c| match c.get_zone() {
                Zone::Realm(s) => {
                    if are_adjacent(s, square) {
                        Some(c)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .map(|c| c.get_id().clone())
            .collect()
    }

    fn get_valid_play_squares(&self, state: &State) -> Vec<u8> {
        let site_squares = state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_site())
            .filter_map(|c| match c.get_zone() {
                Zone::Realm(sq) => Some(sq),
                _ => None,
            })
            .collect();
        site_squares
    }

    fn get_toughness(&self, state: &State) -> Option<u8> {
        self.get_unit_base().map(|ub| ub.toughness)
    }

    fn get_power(&self, state: &State) -> Option<u8> {
        self.get_unit_base().map(|ub| ub.power)
    }

    fn get_required_thresholds(&self, state: &State) -> &Thresholds {
        &self.get_base().required_thresholds
    }

    fn get_mana_cost(&self, state: &State) -> u8 {
        self.get_base().mana_cost
    }

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        None
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        None
    }

    fn get_site_base(&self) -> Option<&SiteBase> {
        None
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        None
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        None
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        None
    }

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

    fn is_avatar(&self) -> bool {
        self.get_card_type() == CardType::Avatar
    }

    fn is_unit(&self) -> bool {
        self.get_card_type() == CardType::Spell
    }

    fn on_move(&mut self, state: &State, zone: &Zone) -> Vec<Effect> {
        vec![]
    }

    fn remove_modifier(&mut self, ability: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.abilities.retain(|a| a != &ability);
        }
    }

    fn add_modifier(&mut self, ability: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.abilities.push(ability);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteBase {
    pub provided_mana: u8,
    pub provided_thresholds: Thresholds,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Modifier {
    Airborne,
    Lethal,
    Movement(u8),
    Burrowing,
    Submerge,
    Spellcaster(Element),
    TakesNoDamageFromElement(Element),
    Charge,
    SummoningSickness,
}

#[derive(Debug, Default, Clone)]
pub struct UnitBase {
    pub power: u8,
    pub toughness: u8,
    pub abilities: Vec<Modifier>,
    pub damage: u8,
}

#[derive(Debug, Clone)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub tapped: bool,
    pub zone: Zone,
    pub mana_cost: u8,
    pub required_thresholds: Thresholds,
}

#[derive(Debug, Clone)]
pub struct AvatarBase {
    pub playing_site: Option<uuid::Uuid>,
}

pub fn from_name(name: &str, player_id: PlayerId) -> Box<dyn Card> {
    if let Some(card) = beta::from_beta_name(name, player_id) {
        return card;
    }

    panic!("Card with name '{}' not found", name);
}

pub fn from_name_and_zone(name: &str, player_id: PlayerId, zone: Zone) -> Box<dyn Card> {
    let mut card = from_name(name, player_id);
    card.set_zone(zone);
    card
}
