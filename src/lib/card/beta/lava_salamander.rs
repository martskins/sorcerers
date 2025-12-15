use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, Modifier, UnitBase, Zone},
    game::{Element, PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct LavaSalamander {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl LavaSalamander {
    pub const NAME: &'static str = "Lava Salamander";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                modifiers: vec![
                    Modifier::Spellcaster(Element::Fire),
                    Modifier::TakesNoDamageFromElement(Element::Fire),
                ],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("FF"),
            },
        }
    }
}

impl Card for LavaSalamander {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Spell
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }
}

impl MessageHandler for LavaSalamander {}
