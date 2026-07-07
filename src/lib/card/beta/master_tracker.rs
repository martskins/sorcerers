use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MasterTracker {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MasterTracker {
    pub const NAME: &'static str = "Master Tracker";
    pub const DESCRIPTION: &'static str = "All enemies permanently lose Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for MasterTracker {
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

    // TODO: Reimplement this as a play minion hook + genesis effect.

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::RemoveAbilities {
            removal: AbilityRemoval::exact(Ability::Stealth),
            affected_cards: Box::new(CardQuery::new()
                .in_play()
                .not_controlled_by(&self.get_controller_id(state))),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MasterTracker::NAME, |owner_id: PlayerId| {
        Box::new(MasterTracker::new(owner_id))
    });
