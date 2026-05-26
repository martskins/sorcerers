use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Flood {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Flood {
    pub const NAME: &'static str = "Flood";
    pub const DESCRIPTION: &'static str = "Affected sites are flooded. They're water sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase { tapped: false },
        }
    }
}

impl Aura for Flood {}

#[async_trait::async_trait]
impl Card for Flood {
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

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![ContinuousEffect::FloodSites {
            affected_sites: CardQuery::new()
                .in_affected_zones_of_card(self.get_id())
                .sites(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Flood::NAME, |owner_id: PlayerId| {
    Box::new(Flood::new(owner_id))
});
