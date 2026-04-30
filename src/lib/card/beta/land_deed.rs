use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct LandDeed {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl LandDeed {
    pub const NAME: &'static str = "Land Deed";
    pub const DESCRIPTION: &'static str = "Bearer controls the site they currently occupy.";

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
                costs: Costs::mana_only(4),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for LandDeed {}

#[async_trait::async_trait]
impl Card for LandDeed {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = self
            .get_artifact()
            .expect("LandDeed should have an artifact base")
            .get_bearer()?;

        let Some(bearer_id) = bearer_id else {
            return Ok(vec![]);
        };

        let bearer = state.get_card(&bearer_id);
        let bearer_zone = bearer.get_zone();
        let site_id = match bearer_zone.get_site(state) {
            Some(site) => *site.get_id(),
            None => return Ok(vec![]),
        };

        Ok(vec![ContinuousEffect::ControllerOverride {
            controller_id: bearer.get_controller_id(state),
            affected_cards: site_id.into(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (LandDeed::NAME, |owner_id: PlayerId| {
    Box::new(LandDeed::new(owner_id))
});
