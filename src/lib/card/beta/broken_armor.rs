use crate::{
    card::{Artifact, ArtifactBase, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct BrokenArmor {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl BrokenArmor {
    pub const NAME: &'static str = "Broken Armor";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                bearer: None,
                needs_bearer: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::ZERO,
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: true,
                ..Default::default()
            },
        }
    }
}

impl Artifact for BrokenArmor {}

#[async_trait::async_trait]
impl Card for BrokenArmor {
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BrokenArmor::NAME, |owner_id: PlayerId| {
    Box::new(BrokenArmor::new(owner_id))
});
