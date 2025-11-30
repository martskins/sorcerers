pub mod avatar;
pub mod site;
pub mod spell;

use crate::{
    card::{
        avatar::Avatar,
        site::Site,
        spell::{Spell, SpellType},
    },
    effect::Effect,
    game::State,
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            Edition::Beta => "bet",
            Edition::ArthurianLegends => "art",
            Edition::Alpha => todo!(),
            Edition::Dragonlord => todo!(),
            Edition::Gothic => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Site,
    Spell,
    Avatar,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardBase {
    pub id: uuid::Uuid,
    pub owner_id: uuid::Uuid,
    pub zone: CardZone,
    pub tapped: bool,
}

impl CardBase {
    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            owner_id,
            zone,
            tapped: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardZone {
    None,
    Hand,
    Spellbook,
    Atlasbook,
    Cemetery,
    Realm(u8),
}

impl Default for CardZone {
    fn default() -> Self {
        CardZone::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    None,
    Cards(Vec<uuid::Uuid>),
    Card(uuid::Uuid),
    Cell(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Card {
    Site(Site),
    Spell(Spell),
    Avatar(Avatar),
}

impl Card {
    pub fn untap(&mut self) {
        let base = self.get_base_mut();
        base.tapped = false;
    }

    pub fn tap(&mut self) {
        let base = self.get_base_mut();
        base.tapped = true;
    }

    pub fn is_tapped(&self) -> bool {
        self.get_base().tapped
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        match self {
            Card::Site(card) => card.get_base_mut(),
            Card::Spell(card) => card.get_base_mut(),
            Card::Avatar(card) => card.get_base_mut(),
        }
    }

    fn get_base(&self) -> &CardBase {
        match self {
            Card::Site(card) => card.get_base(),
            Card::Spell(card) => card.get_base(),
            Card::Avatar(card) => card.get_base(),
        }
    }

    pub fn is_site(&self) -> bool {
        matches!(self, Card::Site(_))
    }

    pub fn is_avatar(&self) -> bool {
        matches!(self, Card::Avatar(_))
    }

    pub fn is_spell(&self) -> bool {
        matches!(self, Card::Spell(_))
    }

    pub fn get_name(&self) -> &str {
        match self {
            Card::Site(site) => site.get_name(),
            Card::Spell(spell) => spell.get_name(),
            Card::Avatar(avatar) => avatar.get_name(),
        }
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

    pub fn get_cell_id(&self) -> Option<u8> {
        match self {
            Card::Site(card) => card.get_cell_id(),
            Card::Spell(card) => card.get_cell_id(),
            Card::Avatar(card) => card.get_cell_id(),
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

    pub fn on_turn_start(&self, state: &State) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.on_turn_start(state),
            Card::Site(card) => card.on_turn_start(state),
            Card::Avatar(card) => card.on_turn_start(state),
        }
    }

    pub fn deathrite(&self) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.deathrite(),
            Card::Site(card) => card.deathrite(),
            Card::Avatar(_card) => vec![],
        }
    }

    pub fn take_damage(&self, from: &uuid::Uuid, amount: u8) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.take_damage(from, amount),
            Card::Site(card) => card.take_damage(from, amount),
            Card::Avatar(card) => card.take_damage(from, amount),
        }
    }

    pub fn on_damage_taken(&self, from: &uuid::Uuid, amount: u8, state: &State) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.on_damage_taken(from, amount, state),
            Card::Site(card) => card.on_damage_taken(from, amount, state),
            Card::Avatar(card) => card.on_damage_taken(from, amount, state),
        }
    }

    pub fn genesis(&self) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.genesis(),
            Card::Site(card) => card.genesis(),
            Card::Avatar(_card) => vec![],
        }
    }

    pub fn on_select(&self, state: &State) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.on_select(state),
            Card::Site(card) => card.on_select(state),
            Card::Avatar(card) => card.on_select(state),
        }
    }

    pub fn on_cast(&self, state: &State, target: Target) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.on_cast(state, target),
            Card::Site(_) => vec![],
            Card::Avatar(_) => vec![],
        }
    }

    pub fn on_prepare(&self, state: &State) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.on_prepare(state),
            Card::Site(card) => card.on_prepare(state),
            Card::Avatar(_) => vec![],
        }
    }

    /// Returns the required thresholds to play the spell.
    pub fn get_required_threshold(&self) -> Thresholds {
        match self {
            Card::Site(_) => Thresholds::zero(),
            Card::Spell(spell) => spell.get_required_threshold(),
            Card::Avatar(_) => Thresholds::zero(),
        }
    }

    pub fn after_resolve(&self, state: &State) -> Vec<Effect> {
        match self {
            Card::Spell(card) => card.after_resolve(state),
            Card::Site(_) => vec![],
            Card::Avatar(_) => vec![],
        }
    }

    pub fn is_minion(&self) -> bool {
        match self {
            Card::Site(_) => false,
            Card::Spell(card) => card.get_spell_type() == SpellType::Minion,
            Card::Avatar(_) => true,
        }
    }

    pub fn is_unit(&self) -> bool {
        match self {
            Card::Site(_) => false,
            Card::Spell(card) => card.is_unit(),
            Card::Avatar(_) => true,
        }
    }

    pub fn get_edition(&self) -> Edition {
        match self {
            Card::Site(site) => site.get_edition(),
            Card::Spell(spell) => spell.get_edition(),
            Card::Avatar(avatar) => avatar.get_edition(),
        }
    }
}
