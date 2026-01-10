pub const ARTIFACT_TEMPLATE: &str = r#"use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    game::{PlayerId, Thresholds},
};

#[derive(Debug, Clone)]
pub struct {StructName} {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl {StructName} {
    pub const NAME: &'static str = "{CardName}";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                bearer: None
                needs_bearer: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new({ManaCost}, "{RequiredThresholds}"),
                plane: Plane::Surface,
                rarity: Rarity::{Rarity},
                edition: Edition::{Edition},
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for {StructName} {}

#[async_trait::async_trait]
impl Card for {StructName} {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = ({StructName}::NAME, |owner_id: PlayerId| {
    Box::new({StructName}::new(owner_id))
});"#;
