use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::{CardMatcher, ContinuousEffect, State},
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
                is_token: false,
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

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let affected_zones = self.get_affected_zones(state);
        Ok(vec![ContinuousEffect::FloodSites {
            affected_sites: CardMatcher::new().in_zones(&affected_zones).card_type(CardType::Site),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Flood::NAME, |owner_id: PlayerId| Box::new(Flood::new(owner_id)));
