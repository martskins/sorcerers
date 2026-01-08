use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct GyreHippogriffs {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl GyreHippogriffs {
    pub const NAME: &'static str = "Gyre Hippogriffs";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                modifiers: vec![Modifier::Airborne, Modifier::Charge],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Air,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GyreHippogriffs {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (GyreHippogriffs::NAME, |owner_id: PlayerId| {
    Box::new(GyreHippogriffs::new(owner_id))
});
