use crate::{
    card::{AvatarBase, Card, CardBase, Edition, Plane, Rarity, UnitBase, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct Battlemage {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
    pub avatar_base: AvatarBase,
}

impl Battlemage {
    pub const NAME: &'static str = "Battlemage";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 0,
                required_thresholds: Thresholds::new(),
                plane: Plane::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            avatar_base: AvatarBase {},
        }
    }
}

impl Card for Battlemage {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }
}
