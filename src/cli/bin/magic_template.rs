pub const MAGIC_TEMPLATE: &str = r#"use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct {StructName} {
    pub card_base: CardBase,
}

impl {StructName} {
    pub const NAME: &'static str = "{CardName}";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: {ManaCost},
                required_thresholds: Thresholds::parse("{RequiredThresholds}"),
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

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    ({StructName}::NAME, |owner_id: PlayerId| Box::new({StructName}::new(owner_id)));"#;
