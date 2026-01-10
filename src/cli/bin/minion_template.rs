pub const MINION_TEMPLATE: &str = r#"use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct {StructName} {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl {StructName} {
    pub const NAME: &'static str = "{CardName}";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: {Power},
                toughness: {Toughness},
                abilities: vec![{Modifiers}],
                types: vec![{MinionTypes}],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new({ManaCost}, "{RequiredThresholds}"),
                plane: Plane::Surface,
                rarity: Rarity::{Rarity},
                edition: Edition::{Edition},
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for {StructName} {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    ({StructName}::NAME, |owner_id: PlayerId| Box::new({StructName}::new(owner_id)));
"#;
