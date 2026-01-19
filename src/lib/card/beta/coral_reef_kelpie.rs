use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct CoralReefKelpie {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl CoralReefKelpie {
    pub const NAME: &'static str = "Coral-Reef Kelpie";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CoralReefKelpie {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (CoralReefKelpie::NAME, |owner_id: PlayerId| {
    Box::new(CoralReefKelpie::new(owner_id))
});
