use std::collections::HashMap;

use crate::{
    card::{
        Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    game::{Direction, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HillockBasilisk {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HillockBasilisk {
    pub const NAME: &'static str = "Hillock Basilisk";
    pub const DESCRIPTION: &'static str =
        "Other minions at rest here or one step in front of Hillock Basilisk are disabled.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Card for HillockBasilisk {
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
            .map(|c| (*c.get_id(), vec![Ability::Disabled]))
            .collect::<HashMap<uuid::Uuid, Vec<Ability>>>();

        AreaModifiers {
            grants_abilities,
            // grants_abilities: vec![(Ability::Disabled, units)],
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (HillockBasilisk::NAME, |owner_id: PlayerId| {
        Box::new(HillockBasilisk::new(owner_id))
    });
