use crate::prelude::*;

const BEARER_ATTACK_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct ScreamingSkull {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl ScreamingSkull {
    pub const NAME: &'static str = "Screaming Skull";
    pub const DESCRIPTION: &'static str = "Whenever bearer attacks and kills an enemy, it untaps.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for ScreamingSkull {}

#[async_trait::async_trait]
impl Card for ScreamingSkull {
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

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let Some(bearer_id) = self.get_bearer()? else {
            return Ok(vec![]);
        };

        let player_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&player_id)?;
        Ok(vec![Hook {
            id: BEARER_ATTACK_HOOK,
            trigger: EffectQuery::UnitKilled {
                unit: CardQuery::new().minions().controlled_by(&opponent_id),
                killer: Some(bearer_id.into()),
                from_attack: Some(true),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            BEARER_ATTACK_HOOK => {
                let Some(bearer_id) = self.get_bearer()? else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::SetTapped {
                    card_id: bearer_id,
                    tapped: false,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScreamingSkull::NAME, |owner_id: PlayerId| {
        Box::new(ScreamingSkull::new(owner_id))
    });
