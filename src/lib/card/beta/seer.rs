use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Seer {
    card_base: CardBase,
    unit_base: UnitBase,
    avatar_base: AvatarBase,
}

impl Seer {
    pub const NAME: &'static str = "Seer";
    pub const DESCRIPTION: &'static str = "Tap → Play or draw a site.\r \r At the start of your turn, look at your topmost site or spell. You may put it on the bottom of its deck.";

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
                rarity: Rarity::Elite,
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
impl Card for Seer {
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

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        // TODO: prompt to inspect topmost site or spell and optionally bottom it.
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Seer::NAME, |owner_id: PlayerId| {
    Box::new(Seer::new(owner_id))
});
