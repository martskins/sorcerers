use rand::seq::IndexedRandom;

use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    game::PlayerId,
    query::{CardQuery, ZoneQuery},
    state::State,
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
                bearer: None,
                needs_bearer: true,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(1, ""),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    fn card_query_override(&self, state: &State, query: &CardQuery) -> anyhow::Result<Option<CardQuery>> {
        match query {
            CardQuery::RandomTarget { possible_targets, .. } => {
                if possible_targets.is_empty() {
                    return Ok(None);
                }

                let targets = vec![
                    possible_targets
                        .choose(&mut rand::rng())
                        .ok_or(anyhow::anyhow!("failed to pick random card"))?
                        .clone(),
                    possible_targets
                        .choose(&mut rand::rng())
                        .ok_or(anyhow::anyhow!("failed to pick random card"))?
                        .clone(),
                ];
                Ok(Some(CardQuery::FromOptions {
                    id: uuid::Uuid::new_v4(),
                    options: targets,
                    prompt: Some("Lucky Charm: Choose a target".to_string()),
                    preview: true,
                }))
            }
            CardQuery::RandomUnitInZone { zone, .. } => {
                let options = zone
                    .get_units(state, None)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<_>>();
                if options.is_empty() {
                    return Ok(None);
                }

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
                Ok(Some(CardQuery::FromOptions {
                    id: uuid::Uuid::new_v4(),
                    options: zones,
                    prompt: Some("Lucky Charm: Choose a unit".to_string()),
                    preview: true,
                }))
            }
            _ => Ok(None),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (LuckyCharm::NAME, |owner_id: PlayerId| {
    Box::new(LuckyCharm::new(owner_id))
});
