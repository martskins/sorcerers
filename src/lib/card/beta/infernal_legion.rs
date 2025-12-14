use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, Modifier, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct InfernalLegion {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl InfernalLegion {
    pub const NAME: &'static str = "Infernal Legion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 6,
                required_thresholds: Thresholds::parse("FFF"),
            },
        }
    }
}

impl Card for InfernalLegion {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Spell
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn deathrite(&self, state: &State) -> Vec<Effect> {
        let units_here: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_zone() == self.get_zone())
            .map(|c| c.get_id().clone())
            .collect();
        let mut effects = Vec::new();
        for unit in units_here {
            effects.push(Effect::TakeDamage {
                card_id: unit,
                from: self.get_id().clone(),
                damage: 3,
            });
        }
        effects
    }

    fn on_turn_end(&mut self, state: &State) -> Vec<Effect> {
        let adjacent_units = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| self.get_zone().is_adjacent(&c.get_zone()))
            .map(|c| c.get_id().clone())
            .collect::<Vec<_>>();
        let mut effects = Vec::new();
        for unit in adjacent_units {
            effects.push(Effect::TakeDamage {
                card_id: unit,
                from: self.get_id().clone(),
                damage: 3,
            });
        }
        effects
    }
}

impl MessageHandler for InfernalLegion {}
