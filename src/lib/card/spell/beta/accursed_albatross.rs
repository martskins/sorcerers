use crate::{
    card::{
        spell::{Ability, SpellBase, SpellType},
        CardBase, CardType, CardZone, Edition,
    },
    effect::{Action, Effect},
    game::{Cell, State},
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccursedAlbatross {
    pub spell: SpellBase,
}

impl AccursedAlbatross {
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
                thresholds: Thresholds::parse("W"),
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
        "Accursed Albatross"
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.owner_id
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        &self.spell.card_base.id
    }

    pub fn on_damage_taken(&self, from: &uuid::Uuid, _amount: u8, state: &State) -> Vec<Effect> {
        if self.spell.damage_taken < self.get_toughness().unwrap_or(0) {
            return vec![];
        }

        let attacker_owner_id = state.cards.iter().find(|c| c.get_id() == from).unwrap().get_owner_id();
        let nearby_minions = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_owner_id() == attacker_owner_id)
            .filter(|c| c.get_id() != from)
            .filter(|c| matches!(c.get_zone(), CardZone::Realm(_)))
            .filter(|c| {
                let a = self.get_cell_id().unwrap();
                let b = c.get_cell_id().unwrap();
                Cell::are_nearby(a, b)
            })
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<uuid::Uuid>>();
        let mut effects = Vec::new();
        for id in nearby_minions {
            effects.push(Effect::KillUnit { card_id: id });
        }
        effects
    }

    pub fn on_select_in_realm_actions(&self, state: &State) -> Vec<Action> {
        vec![]
    }
}
