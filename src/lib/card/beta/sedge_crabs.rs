use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct SedgeCrabs {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SedgeCrabs {
    pub const NAME: &'static str = "Sedge Crabs";

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
                cost: Cost::new(1, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SedgeCrabs {
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
        let mut zones = self.base_valid_move_zones(state)?;
        let crabs_square = self.get_zone().get_square().unwrap_or_default() as i8;
        zones.retain(|z| {
            let zone_square = z.get_square().unwrap_or_default() as i8;
            let diff = crabs_square - zone_square;
            diff % 5 != 0
        });
        Ok(zones)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SedgeCrabs::NAME, |owner_id: PlayerId| {
    Box::new(SedgeCrabs::new(owner_id))
});
