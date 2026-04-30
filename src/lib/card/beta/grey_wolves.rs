use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct GreyWolves {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GreyWolves {
    pub const NAME: &'static str = "Grey Wolves";
    pub const DESCRIPTION: &'static str = "Your spellbook may include any number of Grey Wolves.\r \r Has +1 power for each other Grey Wolves nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GreyWolves {
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

    async fn get_continuous_effects(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let nearby_wolf_count = CardQuery::new()
            .minions()
            .near_to(self.get_zone())
            .id_not(self.get_id())
            .all(state)
            .into_iter()
            .filter(|id| {
                state
                    .get_card(id)
                    .get_name()
                    .eq_ignore_ascii_case(Self::NAME)
            })
            .count() as i16;

        if nearby_wolf_count == 0 {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ModifyPower {
            power_diff: nearby_wolf_count,
            affected_cards: self.get_id().into(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GreyWolves::NAME, |owner_id: PlayerId| {
    Box::new(GreyWolves::new(owner_id))
});
