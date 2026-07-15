use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct PurgeJuggernaut {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl PurgeJuggernaut {
    pub const NAME: &'static str = "Purge Juggernaut";
    pub const DESCRIPTION: &'static str = "At the start of your turn, Purge Juggernaut taps and moves to an adjacent location. Kill all other minions there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                power: Some(4),
                toughness: Some(4),
                types: vec![ArtifactType::Automaton],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(6),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

const TURN_START_HOOK: HookId = 1;

impl Artifact for PurgeJuggernaut {}

#[async_trait::async_trait]
impl Card for PurgeJuggernaut {
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
    fn get_artifact(&self) -> Option<&dyn Artifact> {
        Some(self)
    }
    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.artifact_base)
    }
    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.artifact_base)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_START_HOOK,
            trigger: EffectQuery::TurnStart { player_id: None },
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
            TURN_START_HOOK => {
                let controller_id = self.get_controller_id(state);
                if state.current_player() != controller_id {
                    return Ok(vec![]);
                }
                if !self.get_zone().is_in_play() {
                    return Ok(vec![]);
                }
                let self_id = *self.get_id();
                let target_location = LocationQuery::new()
                    .adjacent_to(self.get_zone())
                    .pick(&controller_id, state)
                    .await?;
                let target_minions: Vec<Effect> = CardQuery::new()
                    .minions()
                    .in_location(target_location.clone())
                    .id_not(*self.get_id())
                    .all(state)
                    .into_iter()
                    .map(|unit_id| Effect::KillMinion {
                        card_id: unit_id,
                        killer_id: self_id,
                        from_attack: false,
                    })
                    .collect();
                let mut effects = vec![Effect::MoveCard {
                    player_id: controller_id,
                    card_id: self_id,
                    from: self.get_location().clone(),
                    to: target_location.into(),
                    tap: true,
                    through_path: None,
                }];
                effects.extend(target_minions);
                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PurgeJuggernaut::NAME, |owner_id: PlayerId| {
        Box::new(PurgeJuggernaut::new(owner_id))
    });
