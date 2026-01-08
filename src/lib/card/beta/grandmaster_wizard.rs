use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct GrandmasterWizard {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl GrandmasterWizard {
    pub const NAME: &'static str = "Grandmaster Wizard";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
                modifiers: vec![
                    Modifier::Spellcaster(Element::Fire),
                    Modifier::Spellcaster(Element::Air),
                    Modifier::Spellcaster(Element::Earth),
                    Modifier::Spellcaster(Element::Water),
                ],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 6,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GrandmasterWizard {
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

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::DrawSpell {
            player_id: self.get_owner_id().clone(),
            count: 3,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (GrandmasterWizard::NAME, |owner_id: PlayerId| {
    Box::new(GrandmasterWizard::new(owner_id))
});
