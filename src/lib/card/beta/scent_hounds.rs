use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ScentHounds {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ScentHounds {
    pub const NAME: &'static str = "Scent Hounds";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                modifiers: vec![],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("E"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ScentHounds {
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

    fn area_effects(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let opponent_id = state.get_opponent_id(&self.get_controller_id())?;
        let effects = self
            .get_zone()
            .get_nearby_units(state, Some(&opponent_id))
            .into_iter()
            .filter(|c| c.has_modifier(state, &Modifier::Stealth))
            .map(|c| Effect::RemoveModifier {
                card_id: c.get_id().clone(),
                modifier: Modifier::Stealth,
            })
            .collect();

        Ok(effects)
    }
}
