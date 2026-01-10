use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MinorExplosion {
    pub card_base: CardBase,
}

impl MinorExplosion {
    pub const NAME: &'static str = "Minor Explosion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "FF"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MinorExplosion {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let caster = state.get_card(caster_id);
        let valid_zones = caster.get_zones_within_steps(state, 2);
        let prompt = "Pick a zone to center Minor Explosion:";
        let zone = pick_zone(self.get_owner_id(), &valid_zones, state, prompt).await?;
        let units = state.get_units_in_zone(&zone);
        Ok(units
            .iter()
            .map(|c| Effect::take_damage(c.get_id(), self.get_id(), 3))
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (MinorExplosion::NAME, |owner_id: PlayerId| {
    Box::new(MinorExplosion::new(owner_id))
});
