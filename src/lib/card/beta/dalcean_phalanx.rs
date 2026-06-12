use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DalceanPhalanx {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DalceanPhalanx {
    pub const NAME: &'static str = "Dalcean Phalanx";
    pub const DESCRIPTION: &'static str = "Can only move themselves forward.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
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
impl Card for DalceanPhalanx {
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

    async fn get_valid_move_locations(&self, state: &State) -> anyhow::Result<Vec<Location>> {
        let locations = self.base_valid_move_locations(state).await?;
        let mut valid_locations = vec![self.get_location().clone()];
        while let Some(location) = valid_locations
            .last()
            .unwrap()
            .step_in_direction(&Direction::Up, state, Some(self.get_id()))
        {
            valid_locations.push(location);
        }

        Ok(locations
            .iter()
            .filter(|z| valid_locations.contains(z))
            .cloned()
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (DalceanPhalanx::NAME, |owner_id: PlayerId| {
        Box::new(DalceanPhalanx::new(owner_id))
    });
