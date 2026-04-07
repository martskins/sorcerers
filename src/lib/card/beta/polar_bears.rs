use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct PolarBears {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl PolarBears {
    pub const NAME: &'static str = "Polar Bears";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn wrapped_neighbours(zone: &Zone) -> Vec<Zone> {
        // Polar Bears can wrap between top row (16-20) and bottom row (1-5)
        match zone {
            Zone::Realm(id) if *id >= 1 && *id <= 5 => vec![Zone::Realm(id + 15)],
            Zone::Realm(id) if *id >= 16 && *id <= 20 => vec![Zone::Realm(id - 15)],
            _ => vec![],
        }
    }
}

#[async_trait::async_trait]
impl Card for PolarBears {
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

    fn get_zones_within_steps_of(&self, state: &State, steps: u8, zone: &Zone) -> Vec<Zone> {
        let mut visited = Vec::new();
        let mut to_visit = vec![(zone.clone(), 0u8)];

        while let Some((current_zone, current_step)) = to_visit.pop() {
            if current_step > steps {
                continue;
            }

            if !visited.contains(&current_zone) {
                visited.push(current_zone.clone());

                for adjacent in current_zone.get_adjacent() {
                    to_visit.push((adjacent, current_step + 1));
                }

                for wrapped in PolarBears::wrapped_neighbours(&current_zone) {
                    to_visit.push((wrapped, current_step + 1));
                }
            }
        }

        if self.is_unit() && !self.has_ability(state, &Ability::Voidwalk) {
            visited = visited
                .iter()
                .filter(|z| z.get_site(state).is_some())
                .cloned()
                .collect();
        }

        visited
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PolarBears::NAME, |owner_id: PlayerId| {
    Box::new(PolarBears::new(owner_id))
});
