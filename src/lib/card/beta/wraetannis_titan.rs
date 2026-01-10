use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct WraetannisTitan {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl WraetannisTitan {
    pub const NAME: &'static str = "Wraetannis Titan";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                modifiers: vec![],
                types: vec![MinionType::Giant],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(7, "EE"),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WraetannisTitan {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (WraetannisTitan::NAME, |owner_id: PlayerId| {
    Box::new(WraetannisTitan::new(owner_id))
});
