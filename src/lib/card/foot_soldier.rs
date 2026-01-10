use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct FootSoldier {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl FootSoldier {
    pub const NAME: &'static str = "Foot Soldier";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Atlasbook,
                cost: Cost::zero(),
                plane: Plane::Surface,
                rarity: Rarity::Token,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for FootSoldier {
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

    fn get_num_arts(&self) -> usize {
        3
    }
}
