use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SwivenScout {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SwivenScout {
    pub const NAME: &'static str = "Swiven Scout";
    pub const DESCRIPTION: &'static str = "Movement +1
Enemy Avatars within Swiven Scout's range of motion play with their hands revealed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Movement(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
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

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let range = self.get_steps_per_movement(state).unwrap_or(0);
        let locations = self.get_locations_within_steps(state, range);
        let avatars = CardQuery::new()
            .avatars()
            .in_locations(&locations)
            // TODO: Use new not_controlled_by
            // .controlled_by_different_controller_than_card(card_id)
            .all(state);
        for avatar_id in avatars {
            let avatar = state.get_card(&avatar_id);
            let hand = CardQuery::new()
                .in_zone(Zone::Hand)
                // TODO: Should be owned by
                .controlled_by(&avatar.get_controller_id(state))
                .all(state);
            if hand.is_empty() {
                continue;
            }

            let _player = state.get_player(avatar.get_owner_id())?;
            // TODO: Add effect to make cards in hand visible
        }

        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SwivenScout::NAME, |owner_id: PlayerId| {
    Box::new(SwivenScout::new(owner_id))
});
