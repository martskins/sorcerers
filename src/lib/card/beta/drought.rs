use crate::{
    card::{Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Drought {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
}

impl Drought {
    pub const NAME: &'static str = "Drought";
    pub const DESCRIPTION: &'static str =
        "Affected sites aren't water sites, and provide no water threshold.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {},
        }
    }
}

impl Aura for Drought {}

#[async_trait::async_trait]
impl Card for Drought {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let affected_zones = self.get_affected_zones(state);
        Ok(vec![ContinuousEffect::DroughtSites {
            affected_sites: CardQuery::new().in_zones(&affected_zones).sites(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Drought::NAME, |owner_id: PlayerId| {
        Box::new(Drought::new(owner_id))
    });
