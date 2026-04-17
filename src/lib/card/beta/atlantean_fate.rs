use crate::{
    card::{Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct AtlanteanFate {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl AtlanteanFate {
    pub const NAME: &'static str = "Atlantean Fate";
    pub const DESCRIPTION: &'static str = "Affected non-Ordinary sites are flooded. They are water sites, only provide Water threshold, and lose all other abilities.\r \r Genesis → Submerge all minions and artifacts atop affected sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "WW"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
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
            .map(|c| *c.get_id())
            .collect()
    }
}

impl Aura for AtlanteanFate {}

#[async_trait::async_trait]
impl Card for AtlanteanFate {
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
        let flooded_sites = self.flooded_site_ids(state);
        if flooded_sites.is_empty() {
            return Ok(vec![]);
        }

        // TODO: This is missing the effect of removing all other abilities and affinities from the
        // affected sites.
        Ok(vec![ContinuousEffect::FloodSites {
            affected_sites: CardQuery::from_ids(flooded_sites),
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
                    card_id: *card.get_id(),
                    region: Region::Underwater,
                    tap: false,
                });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AtlanteanFate::NAME, |owner_id: PlayerId| {
        Box::new(AtlanteanFate::new(owner_id))
    });
