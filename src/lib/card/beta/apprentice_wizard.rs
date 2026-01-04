use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ApprenticeWizard {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl ApprenticeWizard {
    pub const NAME: &'static str = "Apprentice Wizard";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![
                    Modifier::Spellcaster(Element::Air),
                    Modifier::Spellcaster(Element::Fire),
                    Modifier::Spellcaster(Element::Earth),
                    Modifier::Spellcaster(Element::Fire),
                ],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ApprenticeWizard {
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

    async fn genesis(&self, _state: &State) -> Vec<Effect> {
        vec![Effect::DrawSpell {
            player_id: self.get_owner_id().clone(),
            count: 1,
        }]
    }
}
