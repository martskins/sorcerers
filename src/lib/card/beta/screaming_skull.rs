use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct ScreamingSkull {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl ScreamingSkull {
    pub const NAME: &'static str = "Screaming Skull";
    pub const DESCRIPTION: &'static str = "Whenever a unit is buried, untap bearer.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Relic],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for ScreamingSkull {}

#[async_trait::async_trait]
impl Card for ScreamingSkull {
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

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }

    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let skull_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::BuryCard {
                    card: CardQuery::new().units(),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(skull_id),
                }),
                on_effect: Arc::new(move |state: &State, _: &uuid::Uuid, _: &Effect| {
                    Box::pin(async move {
                        let skull = state.get_card(&skull_id);
                        if !skull.get_zone().is_in_play() {
                            return Ok(vec![]);
                        }
                        let bearer_id = skull.get_bearer_id()?;
                        if let Some(bearer_id) = bearer_id {
                            Ok(vec![Effect::UntapCard { card_id: bearer_id }])
                        } else {
                            Ok(vec![])
                        }
                    })
                        as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                }),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScreamingSkull::NAME, |owner_id: PlayerId| {
        Box::new(ScreamingSkull::new(owner_id))
    });
