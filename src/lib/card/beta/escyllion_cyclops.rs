use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct EscyllionCyclops {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl EscyllionCyclops {
    pub const NAME: &'static str = "Escyllion Cyclops";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                modifiers: vec![Modifier::Charge],
                types: vec![MinionType::Monster],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 6,
                required_thresholds: Thresholds::parse("FF"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Card for EscyllionCyclops {
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

    fn on_defend(&self, _: &State, _: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        // Doesn't strike back while defending
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (EscyllionCyclops::NAME, |owner_id: PlayerId| {
    Box::new(EscyllionCyclops::new(owner_id))
});
