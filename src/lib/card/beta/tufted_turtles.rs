use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct TuftedTurtles {
    unit_base: UnitBase,
    card_base: CardBase,
    damage_prevented: bool,
}

impl TuftedTurtles {
    pub const NAME: &'static str = "Tufted Turtles";
    pub const DESCRIPTION: &'static str =
        "The first time Tufted Turtles would take damage each turn, prevent that damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            damage_prevented: false,
        }
    }
}

#[async_trait::async_trait]
impl Card for TuftedTurtles {
    fn get_name(&self) -> &str {
        Self::NAME
    }
    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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

    fn set_data(
        &mut self,
        _data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(damage_prevented) = _data.downcast_ref::<bool>() {
            self.damage_prevented = *damage_prevented;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Tufted Turtles"))
        }
    }

    async fn on_turn_start(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::SetCardData {
            card_id: *self.get_id(),
            data: std::sync::Arc::new(false),
        }])
    }

    async fn replace_effect(
        &self,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Option<Vec<Effect>>> {
        if self.damage_prevented {
            return Ok(None);
        }

        if let Effect::TakeDamage { card_id, .. } = effect
            && card_id == self.get_id()
        {
            return Ok(Some(vec![Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(true),
            }]));
        }

        Ok(None)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TuftedTurtles::NAME, |owner_id: PlayerId| {
        Box::new(TuftedTurtles::new(owner_id))
    });
