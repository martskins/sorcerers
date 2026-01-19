use crate::{
    card::{Card, CardBase, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct WraetannisTitan {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl WraetannisTitan {
    pub const NAME: &'static str = "Wraetannis Titan";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                abilities: vec![],
                types: vec![MinionType::Giant],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(7, "EE"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WraetannisTitan {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let opponent_id = state.get_opponent_id(&self.get_controller_id(state))?;
        let effects = self
            .get_zone()
            .get_units(state, Some(&opponent_id))
            .iter()
            .map(|c| c.get_id())
            .cloned()
            .map(|id| Effect::TakeDamage {
                card_id: id.clone(),
                from: self.get_id().clone(),
                damage: self.get_power(state).unwrap_or_default().unwrap_or_default(),
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (WraetannisTitan::NAME, |owner_id: PlayerId| {
    Box::new(WraetannisTitan::new(owner_id))
});
