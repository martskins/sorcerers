use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs,
        Edition, Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct TruesightCrossbow {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl TruesightCrossbow {
    pub const NAME: &'static str = "Truesight Crossbow";
    pub const DESCRIPTION: &'static str = "Bearer has Ranged and can see stealthed enemies.";

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
                costs: Costs::mana_only(2),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for TruesightCrossbow {}

#[async_trait::async_trait]
impl Card for TruesightCrossbow {
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
        let bearer_id = self
            .get_artifact()
            .expect("TruesightCrossbow should have artifact base")
            .get_bearer()?;

        let Some(bearer_id) = bearer_id else {
            return Ok(vec![]);
        };

        Ok(vec![
            ContinuousEffect::GrantAbility {
                ability: Ability::Ranged(1),
                affected_cards: bearer_id.into(),
            },
            ContinuousEffect::GrantAbility {
                ability: Ability::CanSeeStealthed,
                affected_cards: bearer_id.into(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TruesightCrossbow::NAME, |owner_id: PlayerId| {
        Box::new(TruesightCrossbow::new(owner_id))
    });
