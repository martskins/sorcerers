use crate::{
    card::{Aura, AuraBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardMatcher, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct AtlanteanFate {
    pub aura_base: AuraBase,
    pub card_base: CardBase,
}

impl AtlanteanFate {
    pub const NAME: &'static str = "Atlantean Fate";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(5, "WW"),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {},
        }
    }

    fn flooded_site_ids(&self, state: &State) -> Vec<uuid::Uuid> {
        let affected_zones = self.get_affected_zones(state);
        state
            .cards
            .iter()
            .filter(|c| affected_zones.contains(c.get_zone()))
            .filter(|c| c.is_site())
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_base().rarity != Rarity::Ordinary)
            .map(|c| c.get_id().clone())
            .collect()
    }
}

impl Aura for AtlanteanFate {}

#[async_trait::async_trait]
impl Card for AtlanteanFate {
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
        let flooded_sites = self.flooded_site_ids(state);
        if flooded_sites.is_empty() {
            return Ok(vec![]);
        }

        // TODO: This is missing the effect of removing all other abilities and affinities from the
        // affected sites.
        Ok(vec![ContinuousEffect::FloodSites {
            affected_sites: CardMatcher::from_ids(flooded_sites),
        }])
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();

        for site_id in self.flooded_site_ids(state) {
            let site = state.get_card(&site_id);
            let zone = site.get_zone().clone();
            for card in state.get_cards_in_zone(&zone) {
                if card.get_id() == &site_id {
                    continue;
                }
                if !card.is_minion() && !card.is_artifact() {
                    continue;
                }

                effects.push(Effect::SetCardRegion {
                    card_id: card.get_id().clone(),
                    region: Region::Underwater,
                    tap: false,
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AtlanteanFate::NAME, |owner_id: PlayerId| {
    Box::new(AtlanteanFate::new(owner_id))
});
