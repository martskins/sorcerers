use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct CrownOfTheVictor {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl CrownOfTheVictor {
    pub const NAME: &'static str = "Crown of the Victor";
    pub const DESCRIPTION: &'static str = "Bearer has +3 power if they've ever killed a minion.";

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
                costs: Costs::mana_only(3),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for CrownOfTheVictor {}

#[async_trait::async_trait]
impl Card for CrownOfTheVictor {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = self
            .get_artifact()
            .expect("CrownOfTheVictor should have an artifact base")
            .get_bearer()?;
        if bearer_id.is_none() {
            return Ok(vec![]);
        }
        let bearer_id = bearer_id.expect("value not to be None");
        let has_killed = state.effect_log.iter().find(|le| matches!(*le.effect, Effect::KillMinion { killer_id, .. } if killer_id == bearer_id));

        if has_killed.is_none() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ModifyPower {
            power_diff: 3,
            affected_cards: bearer_id.into(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CrownOfTheVictor::NAME, |owner_id: PlayerId| {
        Box::new(CrownOfTheVictor::new(owner_id))
    });
