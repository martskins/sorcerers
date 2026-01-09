use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct RollingBoulder {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl RollingBoulder {
    pub const NAME: &'static str = "Rolling Boulder";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase { attached_to: None },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 4,
                required_thresholds: Thresholds::parse(""),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for RollingBoulder {}

#[async_trait::async_trait]
impl Card for RollingBoulder {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (RollingBoulder::NAME, |owner_id: PlayerId| {
    Box::new(RollingBoulder::new(owner_id))
});