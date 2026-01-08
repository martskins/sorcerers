use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct SacredScarabs {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl SacredScarabs {
    pub const NAME: &'static str = "Sacred Scarabs";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![Modifier::Airborne],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Air,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SacredScarabs {
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

    fn deathrite(&self, state: &State, from: &Zone) -> Vec<Effect> {
        let units_here: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_zone() == from)
            .map(|c| c.get_id().clone())
            .collect();
        let mut effects = Vec::new();
        for unit in units_here {
            effects.push(Effect::TakeDamage {
                card_id: unit,
                from: self.get_id().clone(),
                damage: 3,
            });
        }
        effects
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SacredScarabs::NAME, |owner_id: PlayerId| {
    Box::new(SacredScarabs::new(owner_id))
});
