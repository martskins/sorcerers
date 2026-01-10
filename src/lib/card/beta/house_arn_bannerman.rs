use std::collections::HashMap;

use crate::{
    card::{AreaModifiers, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    effect::Counter,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct HouseArnBannerman {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl HouseArnBannerman {
    pub const NAME: &'static str = "House Arn Bannerman";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                modifiers: vec![],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "EE"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HouseArnBannerman {
    fn get_name(&self) -> &str {
        Self::NAME
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
        let nearby_allies: Vec<uuid::Uuid> = self
            .get_zone()
            .get_nearby_units(state, Some(self.get_controller_id()))
            .iter()
            .map(|unit| unit.get_id())
            .filter(|id| *id != self.get_id())
            .cloned()
            .collect();

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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HouseArnBannerman::NAME, |owner_id: PlayerId| {
    Box::new(HouseArnBannerman::new(owner_id))
});
