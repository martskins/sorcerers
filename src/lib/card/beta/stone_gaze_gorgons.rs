use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct StoneGazeGorgons {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl StoneGazeGorgons {
    pub const NAME: &'static str = "Stone-gaze Gorgons";
    pub const DESCRIPTION: &'static str =
        "Other minions at rest at adjacent locations are disabled.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Monster],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "EE"),
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
impl Card for StoneGazeGorgons {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let resting_minions = CardQuery::new()
            .minions()
            .adjacent_to(self.get_zone())
            .untapped()
            .all(state);

        Ok(resting_minions
            .into_iter()
            .map(|card_id| ContinuousEffect::GrantAbility {
                ability: Ability::Disabled,
                affected_cards: card_id.into(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (StoneGazeGorgons::NAME, |owner_id: PlayerId| {
        Box::new(StoneGazeGorgons::new(owner_id))
    });
