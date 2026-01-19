use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct InfernalLegion {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl InfernalLegion {
    pub const NAME: &'static str = "Infernal Legion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![],
                types: vec![MinionType::Undead],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(6, "FFF"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for InfernalLegion {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let adjacent_units: Vec<uuid::Uuid> = self
            .get_zone()
            .get_adjacent()
            .iter()
            .flat_map(|z| state.get_units_in_zone(z))
            .map(|c| c.get_id().clone())
            .collect();
        let mut effects = Vec::new();
        for unit in adjacent_units {
            effects.push(Effect::TakeDamage {
                card_id: unit,
                from: self.get_id().clone(),
                damage: 3,
            });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (InfernalLegion::NAME, |owner_id: PlayerId| {
    Box::new(InfernalLegion::new(owner_id))
});
