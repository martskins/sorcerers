use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
};

/// **Phantom Steed** — Exceptional Minion (3 cost, 2/2)
///
/// Movement +2, Voidwalk. May carry an allied minion.
/// TODO: Implement "may carry an allied minion" mechanic.
#[derive(Debug, Clone)]
pub struct PhantomSteed {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PhantomSteed {
    pub const NAME: &'static str = "Phantom Steed";
    pub const DESCRIPTION: &'static str = "Movement +2, Voidwalk\n\nMay carry an allied minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Movement(2), Ability::Voidwalk],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
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
impl Card for PhantomSteed {
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
static CONSTRUCTOR: (&'static str, CardConstructor) = (PhantomSteed::NAME, |owner_id: PlayerId| {
    Box::new(PhantomSteed::new(owner_id))
});
