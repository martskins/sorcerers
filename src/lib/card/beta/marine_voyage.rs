use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone_group},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MarineVoyage {
    pub card_base: CardBase,
}

impl MarineVoyage {
    pub const NAME: &'static str = "Marine Voyage";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "W"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MarineVoyage {
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
        let controller_id = self.get_controller_id(state);
        let bodies_of_water = state.get_bodies_of_water();
        let prompt = "Marine Voyage: Pick a body of water";
        let body_of_water = pick_zone_group(controller_id, &bodies_of_water, state, false, prompt).await?;
        println!("Picked body of water: {:?}", body_of_water);
        // TODO: Implement the actual effect of Marine Voyage
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (MarineVoyage::NAME, |owner_id: PlayerId| {
    Box::new(MarineVoyage::new(owner_id))
});