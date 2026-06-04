use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MagellanGlobe {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MagellanGlobe {
    pub const NAME: &'static str = "Magellan Globe";
    pub const DESCRIPTION: &'static str = "Opposite edges of the realm are connected.";

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
                costs: Costs::mana_only(2),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for MagellanGlobe {}

#[async_trait::async_trait]
impl Card for MagellanGlobe {
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

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![
            OngoingEffect::ConnectTopBottomEdges {
                affected_cards: CardQuery::new().units().in_play(),
            },
            OngoingEffect::ConnectLeftRightEdges {
                affected_cards: CardQuery::new().units().in_play(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MagellanGlobe::NAME, |owner_id: PlayerId| {
        Box::new(MagellanGlobe::new(owner_id))
    });
