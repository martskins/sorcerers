use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs,
        Edition, Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct PoisonousDagger {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl PoisonousDagger {
    pub const NAME: &'static str = "Poisonous Dagger";
    pub const DESCRIPTION: &'static str = "Bearer has Lethal.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Weapon],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, ""),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for PoisonousDagger {}

#[async_trait::async_trait]
impl Card for PoisonousDagger {
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

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = match self.get_bearer_id()? {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Lethal,
            affected_cards: CardQuery::from_id(bearer_id),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PoisonousDagger::NAME, |owner_id: PlayerId| {
        Box::new(PoisonousDagger::new(owner_id))
    });
