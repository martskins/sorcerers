use crate::{
    card::{AridDesert, ClamorOfHarpies, Flamecaller, PitVipers, beta},
    effect::{Counter, Effect, ModifierCounter},
    game::{Element, PlayerId, Thresholds, are_adjacent, are_nearby},
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

impl Zone {
    pub fn is_nearby(&self, other: &Zone) -> bool {
        match (self, other) {
            (Zone::Realm(sq1), Zone::Realm(sq2)) => are_nearby(*sq1, *sq2),
            _ => false,
        }
    }

    pub fn is_adjacent(&self, other: &Zone) -> bool {
        match (self, other) {
            (Zone::Realm(sq1), Zone::Realm(sq2)) => are_adjacent(*sq1, *sq2),
            _ => false,
        }
    }
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
    pub summoning_sickness: bool,
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

    fn has_modifier(&self, state: &State, modifier: Modifier) -> bool {
        if self
            .get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifiers
            .contains(&modifier)
        {
            return true;
        }

        self.get_unit_base()
            .unwrap_or(&UnitBase::default())
            .modifier_counters
            .iter()
            .find(|c| c.modifier == modifier)
            .is_some()
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
        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() == self.get_owner_id())
            .filter(|c| c.is_site())
            .filter(|c| {
                if self.has_modifier(state, Modifier::Airborne) {
                    self.get_zone().is_adjacent(&c.get_zone())
                } else {
                    self.get_zone().is_nearby(&c.get_zone())
                }
            })
            .map(|c| match c.get_zone() {
                Zone::Realm(sq) => sq,
                _ => unreachable!(),
            })
            .collect()
    }

    fn get_valid_attack_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        state
            .cards
            .iter()
            .filter(|c| c.get_owner_id() != self.get_owner_id())
            .filter(|c| c.is_unit() || c.is_site())
            .filter(|c| c.get_zone().is_adjacent(&self.get_zone()))
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
        let base = self.get_unit_base();
        if base.is_none() {
            return None;
        }

        let base = base.unwrap();
        let mut toughness = base.toughness;
        for counter in &base.power_counters {
            toughness = toughness.saturating_sub_signed(counter.toughness);
        }
        Some(toughness)
    }

    fn get_power(&self, state: &State) -> Option<u8> {
        let base = self.get_unit_base();
        if base.is_none() {
            return None;
        }

        let base = base.unwrap();
        let mut power = base.power;
        for counter in &base.power_counters {
            power = power.saturating_sub_signed(counter.power);
        }
        Some(power)
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
        self.get_site_base().is_some()
    }

    fn is_avatar(&self) -> bool {
        self.get_avatar_base().is_some()
    }

    fn is_unit(&self) -> bool {
        self.get_unit_base().is_some()
    }

    fn is_spell(&self) -> bool {
        self.get_card_type() == CardType::Spell
    }

    fn can_cast(&self, state: &State, spell: &Box<dyn Card>) -> bool {
        if !matches!(self.get_zone(), Zone::Realm(_)) {
            return false;
        }

        if self.get_owner_id() != spell.get_owner_id() {
            return false;
        }

        if self.is_avatar() {
            return true;
        }

        let elements = spell.get_elements(state);
        for element in elements {
            if self.has_modifier(state, Modifier::Spellcaster(element)) {
                return true;
            }
        }

        false
    }

    fn on_move(&mut self, state: &State, zone: &Zone) -> Vec<Effect> {
        vec![]
    }

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> Vec<Effect> {
        let mut effects = Vec::new();
        let attacker = state.cards.iter().find(|c| c.get_id() == from).unwrap();
        if self.is_unit() {
            if let Some(ub) = self.get_unit_base_mut() {
                ub.damage += damage;
            }

            if attacker.has_modifier(state, Modifier::Lethal) {
                effects.push(Effect::BuryCard {
                    card_id: self.get_id().clone(),
                });
            }
        } else if self.is_site() {
            effects.push(Effect::RemoveResources {
                player_id: self.get_owner_id().clone(),
                mana: 0,
                thresholds: Thresholds::new(),
                health: damage,
            });
        } else if self.is_avatar() {
            effects.push(Effect::RemoveResources {
                player_id: self.get_owner_id().clone(),
                mana: 0,
                thresholds: Thresholds::new(),
                health: damage,
            });
        }

        effects
    }

    fn on_turn_end(&mut self, state: &State) -> Vec<Effect> {
        vec![]
    }

    fn remove_modifier(&mut self, modifier: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.retain(|a| a != &modifier);
        }
    }

    fn add_modifier(&mut self, modifier: Modifier) {
        if let Some(ub) = self.get_unit_base_mut() {
            ub.modifiers.push(modifier);
        }
    }

    fn on_cast(&mut self, state: &State) -> Vec<Effect> {
        vec![]
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
    Landbound,
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
    pub modifiers: Vec<Modifier>,
    pub damage: u8,
    pub power_counters: Vec<Counter>,
    pub modifier_counters: Vec<ModifierCounter>,
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
