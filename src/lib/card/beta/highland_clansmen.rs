use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::PlayerId,
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
                abilities: vec![Ability::Charge],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(7, "A"),
                plane: Plane::Air,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HighlandClansmen::NAME, |owner_id: PlayerId| {
    Box::new(HighlandClansmen::new(owner_id))
});
