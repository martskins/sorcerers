use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct RulerOfThul {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl RulerOfThul {
    pub const NAME: &'static str = "Ruler of Thul";
    pub const DESCRIPTION: &'static str = "Charge Allies can move as if the top and bottom edges of the realm were connected. Other allies occupying sites there have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Charge],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RulerOfThul {
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
        let controller_id = self.get_controller_id(state);
        Ok(vec![
            ContinuousEffect::ConnectTopBottomEdges {
                affected_cards: CardQuery::new().units().controlled_by(&controller_id),
            },
            ContinuousEffect::ModifyPower {
                power_diff: 1,
                affected_cards: CardQuery::new()
                    .units()
                    .controlled_by(&controller_id)
                    .id_not(self.get_id())
                    .in_zone(self.get_zone()),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (RulerOfThul::NAME, |owner_id: PlayerId| {
    Box::new(RulerOfThul::new(owner_id))
});
