use std::collections::HashMap;

use crate::{
    card::{Ability, AreaModifiers, Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::{Direction, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HillockBasilisk {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl HillockBasilisk {
    pub const NAME: &'static str = "Hillock Basilisk";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "F"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for HillockBasilisk {
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
        let mut zones = vec![self.get_zone().clone()];
        let board_flipped = self.get_owner_id() != &state.player_one;
        let zone_in_front = self
            .get_zone()
            .zone_in_direction(&Direction::Up.normalise(board_flipped), 1);
        if let Some(zone) = zone_in_front {
            zones.push(zone);
        }

        let grants_abilities = zones
            .iter()
            .flat_map(|z| state.get_units_in_zone(z))
            .filter(|c| c.get_id() != self.get_id())
            .map(|c| (c.get_id().clone(), vec![Ability::Disabled]))
            .collect::<HashMap<uuid::Uuid, Vec<Ability>>>();

        AreaModifiers {
            grants_abilities: grants_abilities,
            // grants_abilities: vec![(Ability::Disabled, units)],
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (HillockBasilisk::NAME, |owner_id: PlayerId| {
    Box::new(HillockBasilisk::new(owner_id))
});
