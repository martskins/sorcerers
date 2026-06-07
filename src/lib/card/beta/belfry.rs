use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Belfry {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl Belfry {
    pub const NAME: &'static str = "Belfry";
    pub const DESCRIPTION: &'static str = "At the end of your turn, untap all nearby allies.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
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

impl Artifact for Belfry {}

#[async_trait::async_trait]
impl Card for Belfry {
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
                if self.get_controller_id(state) != state.current_player() {
                    return Ok(vec![]);
                }

                Ok(CardQuery::new()
                    .units()
                    .near_to(self.get_zone())
                    .all(state)
                    .into_iter()
                    .filter(|card_id| {
                        state.get_card(card_id).get_controller_id(state)
                            == self.get_controller_id(state)
                    })
                    .map(|card_id| Effect::SetTapped {
                        card_id,
                        tapped: false,
                    })
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Belfry::NAME, |owner_id: PlayerId| {
    Box::new(Belfry::new(owner_id))
});
