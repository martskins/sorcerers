use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct AngelsEgg {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl AngelsEgg {
    pub const NAME: &'static str = "Angel's Egg";
    pub const DESCRIPTION: &'static str =
        "At the end of each turn, the controller of Angel's Egg's site heals 1 life";

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

impl Artifact for AngelsEgg {}

#[async_trait::async_trait]
impl Card for AngelsEgg {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd { player_id: None },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_END_HOOK => {
                let site: &dyn Site = match self.get_location().get_site(state) {
                    Some(site) => site,
                    None => return Ok(vec![]),
                };

                Ok(vec![Effect::AdjustAvatarLife {
                    player_id: site.get_controller_id(state),
                    amount: 1,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (AngelsEgg::NAME, |owner_id: PlayerId| {
    Box::new(AngelsEgg::new(owner_id))
});
