use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Rarity, Region, ResourceProvider, Zone},
    game::{Element, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct OnyxCore {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl OnyxCore {
    pub const NAME: &'static str = "Onyx Core";

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
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

impl ResourceProvider for OnyxCore {
    fn provided_mana(&self, _state: &State) -> anyhow::Result<u8> {
        Ok(1)
    }

    fn provides_threshold(&self, element: &Element) -> anyhow::Result<u8> {
        match element {
            Element::Earth => Ok(1),
            _ => Ok(0),
        }
    }
}

impl Artifact for OnyxCore {}

#[async_trait::async_trait]
impl Card for OnyxCore {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (OnyxCore::NAME, |owner_id: PlayerId| Box::new(OnyxCore::new(owner_id)));
