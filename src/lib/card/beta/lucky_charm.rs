use rand::seq::IndexedRandom;

use crate::{
    card::{Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct LuckyCharm {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl LuckyCharm {
    pub const NAME: &'static str = "Lucky Charm";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Relic],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(1),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for LuckyCharm {}

#[async_trait::async_trait]
impl Card for LuckyCharm {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    fn zone_query_override(&self, _state: &State, query: &ZoneQuery) -> anyhow::Result<Option<ZoneQuery>> {
        match query {
            ZoneQuery::Random { options, .. } => {
                let zones = vec![
                    options
                        .choose(&mut rand::rng())
                        .ok_or(anyhow::anyhow!("failed to pick random card"))?
                        .clone(),
                    options
                        .choose(&mut rand::rng())
                        .ok_or(anyhow::anyhow!("failed to pick random card"))?
                        .clone(),
                ];
                Ok(Some(ZoneQuery::FromOptions {
                    id: uuid::Uuid::new_v4(),
                    options: zones,
                    prompt: Some("Lucky Charm: Choose a zone".to_string()),
                }))
            }
            _ => Ok(None),
        }
    }

    async fn card_query_override(&self, state: &State, query: &CardQuery) -> anyhow::Result<Option<CardQuery>> {
        if !query.is_randomised() {
            return Ok(None);
        }

        let query = query.clone();
        let options = query.all(state).choose_multiple(&mut rand::rng(), 2).cloned().collect();
        Ok(Some(
            CardQuery::from_ids(options).with_prompt("Lucky Charm: Choose a card to override random decision"),
        ))
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (LuckyCharm::NAME, |owner_id: PlayerId| {
    Box::new(LuckyCharm::new(owner_id))
});
