use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct PayloadTrebuchet {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl PayloadTrebuchet {
    pub const NAME: &'static str = "Payload Trebuchet";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase { attached_to: None },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(5, ""),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for PayloadTrebuchet {}

#[async_trait::async_trait]
impl Card for PayloadTrebuchet {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PayloadTrebuchet::NAME, |owner_id: PlayerId| {
    Box::new(PayloadTrebuchet::new(owner_id))
});