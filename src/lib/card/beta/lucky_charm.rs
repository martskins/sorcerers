use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LuckyCharm {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl LuckyCharm {
    pub const NAME: &'static str = "Lucky Charm";
    pub const DESCRIPTION: &'static str = "Bearer's controller has “Whenever you would determine an outcome at random, determine it an extra time and choose one.”";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(1),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for LuckyCharm {}

#[async_trait::async_trait]
impl Card for LuckyCharm {
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

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![
            OngoingEffect::choose_from_random_card_options(
                *self.get_id(),
                "Lucky Charm: Choose a card",
                2,
            ),
            OngoingEffect::choose_from_random_zone_options(
                *self.get_id(),
                "Lucky Charm: Choose a zone",
                2,
            ),
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (LuckyCharm::NAME, |owner_id: PlayerId| {
    Box::new(LuckyCharm::new(owner_id))
});
