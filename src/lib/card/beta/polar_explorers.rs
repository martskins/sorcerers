use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct PolarExplorers {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PolarExplorers {
    pub const NAME: &'static str = "Polar Explorers";
    pub const DESCRIPTION: &'static str =
        "Allied units here can move as if the top and bottom edges of the realm were connected.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "WW"),
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
impl Card for PolarExplorers {
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

        Ok(vec![ContinuousEffect::ConnectTopBottomEdges {
            affected_cards: CardQuery::new()
                .units()
                .controlled_by(&self.get_controller_id(state))
                .in_zone(self.get_zone()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PolarExplorers::NAME, |owner_id: PlayerId| {
        Box::new(PolarExplorers::new(owner_id))
    });
