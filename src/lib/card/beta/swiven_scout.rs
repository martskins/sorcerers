use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct SwivenScout {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SwivenScout {
    pub const NAME: &'static str = "Swiven Scout";
    pub const DESCRIPTION: &'static str = "Movement 1.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Movement(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SwivenScout {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SwivenScout::NAME, |owner_id: PlayerId| {
    Box::new(SwivenScout::new(owner_id))
});
