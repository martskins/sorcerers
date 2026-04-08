use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Costs, Edition, Rarity, Region, ResourceProvider, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct AquamarineCore {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl AquamarineCore {
    pub const NAME: &'static str = "Aquamarine Core";

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
                costs: Costs::from_mana_and_threshold(1, ""),
                region: Region::Surface,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl ResourceProvider for AquamarineCore {
    fn provided_mana(&self, _state: &State) -> anyhow::Result<u8> {
        Ok(1)
    }

    fn provided_affinity(&self, _state: &State) -> anyhow::Result<Thresholds> {
        Ok(Thresholds::parse("W"))
    }
}

impl Artifact for AquamarineCore {}

#[async_trait::async_trait]
impl Card for AquamarineCore {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AquamarineCore::NAME, |owner_id: PlayerId| {
    Box::new(AquamarineCore::new(owner_id))
});
