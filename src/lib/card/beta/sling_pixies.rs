use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct SlingPixies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SlingPixies {
    pub const NAME: &'static str = "Sling Pixies";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![Modifier::Airborne, Modifier::Ranged],
                types: vec![MinionType::Fairy],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 1,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Air,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SlingPixies {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> anyhow::Result<Vec<Effect>> {
        let dealer = state.get_card(from);
        if dealer.get_power(state)?.unwrap_or(0) >= 4 {
            // Takes no damage from units with 4 or more power:w
            return Ok(vec![]);
        }

        self.base_take_damage(state, from, damage)
    }
}
