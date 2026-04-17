use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region,
        ResourceProvider, Zone,
    },
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct OnyxCore {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl OnyxCore {
    pub const NAME: &'static str = "Onyx Core";
    pub const DESCRIPTION: &'static str = "Provides (E) and ① to its controller.";

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
                costs: Costs::mana_only(1),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl ResourceProvider for OnyxCore {
    fn provided_mana(&self, _state: &State) -> anyhow::Result<u8> {
        Ok(1)
    }

    fn provided_affinity(&self, _state: &State) -> anyhow::Result<Thresholds> {
        Ok(Thresholds::parse("E"))
    }
}

impl Artifact for OnyxCore {}

#[async_trait::async_trait]
impl Card for OnyxCore {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (OnyxCore::NAME, |owner_id: PlayerId| {
    Box::new(OnyxCore::new(owner_id))
});
