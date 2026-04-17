use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone_group},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MarineVoyage {
    card_base: CardBase,
}

impl MarineVoyage {
    pub const NAME: &'static str = "Marine Voyage";
    pub const DESCRIPTION: &'static str = "This turn, your units can move between any sites in a chosen body of water as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MarineVoyage {
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let bodies_of_water = state.get_bodies_of_water();
        let prompt = "Marine Voyage: Pick a body of water";
        let body_of_water =
            pick_zone_group(controller_id, &bodies_of_water, state, false, prompt).await?;
        println!("Picked body of water: {:?}", body_of_water);
        // TODO: Implement the actual effect of Marine Voyage
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MarineVoyage::NAME, |owner_id: PlayerId| {
    Box::new(MarineVoyage::new(owner_id))
});
