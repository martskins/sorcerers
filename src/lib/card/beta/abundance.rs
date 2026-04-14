use crate::{
    card::{Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Abundance {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
}

impl Abundance {
    pub const NAME: &'static str = "Abundance";
    pub const DESCRIPTION: &'static str = "Each affected site provides one additional mana.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for Abundance {}

#[async_trait::async_trait]
impl Card for Abundance {
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
    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let affected_zones = self.get_affected_zones(state);
        Ok(vec![ContinuousEffect::ModifyProvidedMana {
            mana_diff: 1,
            affected_cards: CardQuery::new().in_zones(&affected_zones).sites(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Abundance::NAME, |owner_id: PlayerId| {
        Box::new(Abundance::new(owner_id))
    });
