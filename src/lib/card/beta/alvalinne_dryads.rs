use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, ResourceProvider, UnitBase, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct AlvalinneDryads {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AlvalinneDryads {
    pub const NAME: &'static str = "Älvalinne Dryads";
    pub const DESCRIPTION: &'static str = "Älvalinne Dryads provide ①.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl ResourceProvider for AlvalinneDryads {
    fn provided_mana(&self, _state: &State) -> anyhow::Result<u8> {
        Ok(1)
    }

    fn provided_affinity(&self, _state: &State) -> anyhow::Result<Thresholds> {
        Ok(Thresholds::ZERO)
    }
}

#[async_trait::async_trait]
impl Card for AlvalinneDryads {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AlvalinneDryads::NAME, |owner_id: PlayerId| {
    Box::new(AlvalinneDryads::new(owner_id))
});
