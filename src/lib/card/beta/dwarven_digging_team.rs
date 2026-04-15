use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct DwarvenDiggingTeam {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DwarvenDiggingTeam {
    pub const NAME: &'static str = "Dwarven Digging Team";
    pub const DESCRIPTION: &'static str =
        "Burrowing\r Allied minions occupying nearby sites have Burrowing.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Burrowing],
                types: vec![MinionType::Dwarf],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "EE"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for DwarvenDiggingTeam {
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

        let controller_id = self.get_controller_id(state);

        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Burrowing,
            affected_cards: CardQuery::new()
                .minions()
                .near_to(self.get_zone())
                .controlled_by(&controller_id)
                .id_not_in(vec![self.get_id().clone()]),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DwarvenDiggingTeam::NAME, |owner_id: PlayerId| {
        Box::new(DwarvenDiggingTeam::new(owner_id))
    });
