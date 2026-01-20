use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct SunkenTreasure {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl SunkenTreasure {
    pub const NAME: &'static str = "Sunken Treasure";

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
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for SunkenTreasure {}

#[async_trait::async_trait]
impl Card for SunkenTreasure {
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

    // TODO: Implement the effect of Sunken Treasure
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SunkenTreasure::NAME, |owner_id: PlayerId| {
    Box::new(SunkenTreasure::new(owner_id))
});
