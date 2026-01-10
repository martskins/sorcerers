use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::{Direction, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct DalceanPhalanx {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl DalceanPhalanx {
    pub const NAME: &'static str = "Dalcean Phalanx";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![],
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
impl Card for DalceanPhalanx {
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

    fn get_valid_move_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        let zones = self.base_valid_move_zones(state)?;
        let mut valid_zones = vec![self.get_zone().clone()];
        while let Some(zone) = valid_zones.last().unwrap().zone_in_direction(&Direction::Up, 1) {
            valid_zones.push(zone);
        }

        Ok(zones.iter().filter(|z| valid_zones.contains(z)).cloned().collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (DalceanPhalanx::NAME, |owner_id: PlayerId| {
    Box::new(DalceanPhalanx::new(owner_id))
});
