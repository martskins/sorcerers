use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

/// Tracks how many more turns the controller-swap effect is active.
#[derive(Debug, Clone, Default)]
struct ThaisData {
    turns_remaining: u8,
}

#[derive(Debug, Clone)]
pub struct CourtesanThais {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    data: ThaisData,
}

impl CourtesanThais {
    pub const NAME: &'static str = "Courtesan Thaïs";
    pub const DESCRIPTION: &'static str =
        "Genesis → During their next turn, each player is controlled by the previous one.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 3,
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            data: ThaisData::default(),
        }
    }
}

#[async_trait::async_trait]
impl Card for CourtesanThais {
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(d) = data.downcast_ref::<ThaisData>() {
            self.data = d.clone();
        }
        Ok(())
    }

    /// Genesis: activate the controller-swap for the next 2 player turns (one round).
    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(ThaisData { turns_remaining: 2 }),
        }])
    }

    /// At the end of each turn, decrement the counter. The override lasts for 2 player turns.
    async fn on_turn_end(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        if self.data.turns_remaining == 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::SetCardData {
            card_id: self.get_id().clone(),
            data: Box::new(ThaisData {
                turns_remaining: self.data.turns_remaining.saturating_sub(1),
            }),
        }])
    }

    /// While turns_remaining > 0, each player's minions are controlled by their opponent.
    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if self.data.turns_remaining == 0 || !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let Ok(opponent_id) = state.get_opponent_id(&controller_id) else {
            return Ok(vec![]);
        };

        // TODO: This card should make the player conrol the opponents turn, not just change the
        // controller of their cards.
        // Each player's minions and avatars are controlled by the opponent.
        Ok(vec![
            ContinuousEffect::ControllerOverride {
                controller_id: opponent_id.clone(),
                affected_cards: CardQuery::new().units().controlled_by(&controller_id),
            },
            ContinuousEffect::ControllerOverride {
                controller_id: controller_id.clone(),
                affected_cards: CardQuery::new().units().controlled_by(&opponent_id),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (CourtesanThais::NAME, |owner_id: PlayerId| {
        Box::new(CourtesanThais::new(owner_id))
    });
