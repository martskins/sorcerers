use std::collections::HashMap;

use crate::{
    card::{
        AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Counter,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct HouseArnBannerman {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HouseArnBannerman {
    pub const NAME: &'static str = "House Arn Bannerman";
    pub const DESCRIPTION: &'static str = "Other nearby allies have +1 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
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
impl Card for HouseArnBannerman {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let nearby_allies = CardQuery::new()
            .units()
            .near_to(self.get_zone())
            .controlled_by(&self.get_controller_id(state))
            .id_not_in(vec![*self.get_id()])
            .all(state);

        let counters: HashMap<uuid::Uuid, Vec<Counter>> = nearby_allies
            .into_iter()
            .map(|unit_id| {
                (
                    unit_id,
                    vec![Counter {
                        id: uuid::Uuid::new_v4(),
                        power: 1,
                        toughness: 0,
                        expires_on_effect: None,
                    }],
                )
            })
            .collect();
        AreaModifiers {
            grants_counters: counters,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HouseArnBannerman::NAME, |owner_id: PlayerId| {
        Box::new(HouseArnBannerman::new(owner_id))
    });
