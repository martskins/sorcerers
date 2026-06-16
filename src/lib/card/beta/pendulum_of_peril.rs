use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct PendulumOfPeril {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl PendulumOfPeril {
    pub const NAME: &'static str = "Pendulum of Peril";
    pub const DESCRIPTION: &'static str = "At the end of each player's turn, Pendulum of Peril kills all minions at its current location and another adjacent location of that player's choice.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
                tapped: false,
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

impl Artifact for PendulumOfPeril {}

#[async_trait::async_trait]
impl Card for PendulumOfPeril {
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
                let zone = self.get_zone();
                if !zone.is_in_play() {
                    return Ok(vec![]);
                }

                let current_player = state.current_player();
                let location = self.get_location();
                let adjacent_locations: Vec<Location> = location
                    .get_adjacent(state)
                    .into_iter()
                    .filter(|adjacent| adjacent != location)
                    .collect();
                let chosen_zone = if adjacent_locations.is_empty() {
                    None
                } else {
                    Some(
                        pick_location(
                            &current_player,
                            &adjacent_locations,
                            state,
                            false,
                            "Pendulum of Peril: Pick an adjacent location to destroy all minions",
                        )
                        .await?,
                    )
                };

                let mut minions = CardQuery::new().minions().in_zone(zone).all(state);
                if let Some(chosen_zone) = chosen_zone {
                    minions.extend(CardQuery::new().minions().in_zone(&chosen_zone).all(state));
                }
                minions.sort();
                minions.dedup();

                Ok(minions
                    .into_iter()
                    .map(|card_id| Effect::KillMinion {
                        card_id,
                        killer_id: *self.get_id(),
                        from_attack: false,
                    })
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PendulumOfPeril::NAME, |owner_id: PlayerId| {
        Box::new(PendulumOfPeril::new(owner_id))
    });
