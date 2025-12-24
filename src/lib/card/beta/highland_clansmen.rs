use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct HighlandClansmen {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl HighlandClansmen {
    pub const NAME: &'static str = "Highland Clansmen";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                modifiers: vec![Modifier::Charge],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 7,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Air,
                rarity: Rarity::Ordinary,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HighlandClansmen {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }
}
