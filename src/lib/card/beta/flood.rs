use crate::{
    card::{Aura, AuraBase, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Flood {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
}

impl Flood {
    pub const NAME: &'static str = "Flood";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "F"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
            aura_base: AuraBase {},
        }
    }
}

impl Aura for Flood {}

#[async_trait::async_trait]
impl Card for Flood {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_valid_play_zones(&self, _state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok(Zone::all_realm())
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    // TODO: Implement Flood effect
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Flood::NAME, |owner_id: PlayerId| Box::new(Flood::new(owner_id)));
