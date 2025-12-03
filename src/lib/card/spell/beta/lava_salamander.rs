use crate::{
    card::{
        spell::{Ability, SpellBase, SpellType},
        CardBase, CardType, CardZone, Combat, Edition, Element, Interaction, Lifecycle, Thresholds,
    },
    effect::{Action, Effect},
    game::State,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LavaSalamander {
    pub spell: SpellBase,
}

impl LavaSalamander {
    pub const NAME: &'static str = "Lava Salamander";

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
                mana_cost: 2,
                thresholds: Thresholds::parse("FF"),
                power: Some(1),
                toughness: Some(1),
            },
        }
    }

    pub fn get_spell_type(&self) -> &SpellType {
        &SpellType::Minion
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
        vec![
            Ability::Spellcaster(Some(vec![Element::Fire])),
            Ability::ImmuneToSpells(Some(vec![Element::Fire])),
        ]
    }

    pub fn get_spell_base(&self) -> &SpellBase {
        &self.spell
    }

    pub fn get_spell_base_mut(&mut self) -> &mut SpellBase {
        &mut self.spell
    }

    pub fn get_square(&self) -> Option<u8> {
        match self.spell.card_base.zone {
            CardZone::Realm(square) => Some(square),
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

impl Lifecycle for LavaSalamander {}
impl Combat for LavaSalamander {}
impl Interaction for LavaSalamander {}
