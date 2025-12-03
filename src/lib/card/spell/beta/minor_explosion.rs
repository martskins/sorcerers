use serde::{Deserialize, Serialize};
use crate::{
    card::{
        spell::{Ability, SpellBase, SpellType},
        CardBase, CardType, CardZone, Combat, Edition, Interaction, Lifecycle, Thresholds,
    },
    effect::{Action, Effect},
    game::State,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinorExplosion {
    pub spell: SpellBase,
}

impl MinorExplosion {
    pub const NAME: &'static str = "Minor Explosion";

    pub fn new(owner_id: uuid::Uuid, zone: CardZone) -> Self {
        Self {
            spell: SpellBase {
                card_base: CardBase {
                    id: uuid::Uuid::new_v4(),
                    owner_id,
                    zone,
                    tapped: false,
                    edition: Edition::Beta,
                },
                damage_taken: 0,
                mana_cost: 3,
                thresholds: Thresholds::parse("FF"),
                power: None,
                toughness: None,
            },
        }
    }

    pub fn get_spell_type(&self) -> &SpellType {
        &SpellType::Magic
    }

    pub fn get_edition(&self) -> &Edition {
        &Edition::Beta
    }

    pub fn get_type(&self) -> CardType {
        CardType::Spell
    }

    pub fn get_toughness(&self) -> Option<u8> {
        self.spell.toughness
    }

    pub fn get_power(&self) -> Option<u8> {
        self.spell.power
    }

    pub fn get_abilities(&self) -> Vec<Ability> {
        vec![]
    }

    pub fn get_spell_base(&self) -> &SpellBase {
        &self.spell
    }

    pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
        &mut self.spell
    }

    pub fn get_cell_id(&self) -> Option<u8> {
        match self.spell.card_base.zone {
            CardZone::Realm(cell_id) => Some(cell_id),
            _ => None,
        }
    }

    pub fn get_name(&self) -> &str {
        Self::NAME
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.owner_id
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.id
    }
}

impl Lifecycle for MinorExplosion {}
impl Combat for MinorExplosion {}
impl Interaction for MinorExplosion {}
