use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct FlamingSword {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl FlamingSword {
    pub const NAME: &'static str = "Flaming Sword";
    pub const DESCRIPTION: &'static str = "Bearer has +1 power, and its strikes splash full damage to each other enemy at a struck unit's location.";

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
                costs: Costs::mana_only(4),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for FlamingSword {}

#[async_trait::async_trait]
impl Card for FlamingSword {
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
        // TODO: Implement the splash damage effect. This will likely require a new ability that is
        // granted by this artifact and the ability should specify how the damage is splashed (e.g.
        // "full damage" vs "half damage", and which enemies are affected).
        let bearer_id = self
            .get_artifact()
            .expect("FlamingSword has artifact base")
            .get_bearer()?;
        match bearer_id {
            None => Ok(vec![]),
            Some(bid) => Ok(vec![ContinuousEffect::ModifyPower {
                power_diff: 1,
                affected_cards: bid.into(),
            }]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FlamingSword::NAME, |owner_id: PlayerId| {
    Box::new(FlamingSword::new(owner_id))
});
