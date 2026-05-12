use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Pathfinder {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Pathfinder {
    pub const NAME: &'static str = "Pathfinder";
    pub const DESCRIPTION: &'static str = "Your atlas can’t contain duplicates. Draw no sites during setup.\r \r Tap → If able, play the topmost site of your atlas to an adjacent location and move there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 20,
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            avatar_base: AvatarBase {
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Pathfinder {
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

    fn get_avatar_base(&self) -> Option<&AvatarBase> {
        Some(&self.avatar_base)
    }

    fn get_avatar_base_mut(&mut self) -> Option<&mut AvatarBase> {
        Some(&mut self.avatar_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        // TODO: add Pathfinder's atlas setup restriction and topmost-site play/move action.
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Pathfinder::NAME, |owner_id: PlayerId| {
    Box::new(Pathfinder::new(owner_id))
});
