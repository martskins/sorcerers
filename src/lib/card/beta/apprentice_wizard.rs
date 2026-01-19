use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{Element, PlayerId},
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
                abilities: vec![
                    Ability::Spellcaster(Element::Air),
                    Ability::Spellcaster(Element::Fire),
                    Ability::Spellcaster(Element::Earth),
                    Ability::Spellcaster(Element::Fire),
                ],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "A"),
                region: Region::Surface,
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

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::DrawSpell {
            player_id: self.get_owner_id().clone(),
            count: 1,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (ApprenticeWizard::NAME, |owner_id| {
    Box::new(ApprenticeWizard::new(owner_id))
});
