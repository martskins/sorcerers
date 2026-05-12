use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct RecurringSpecter {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RecurringSpecter {
    pub const NAME: &'static str = "Recurring Specter";
    pub const DESCRIPTION: &'static str = "Can't defend.\r \r May be cast from your cemetery.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Spirit],
                abilities: vec![Ability::CannotDefend],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RecurringSpecter {
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

    fn is_playable(&self, _state: &State, _player_id: &PlayerId) -> anyhow::Result<bool> {
        Ok(self.get_zone() == &Zone::Hand || self.get_zone() == &Zone::Cemetery)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (RecurringSpecter::NAME, |owner_id: PlayerId| {
        Box::new(RecurringSpecter::new(owner_id))
    });
