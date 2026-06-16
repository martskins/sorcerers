use crate::prelude::*;

const CARD_DRAW_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct ChainsOfPrometheus {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl ChainsOfPrometheus {
    pub const NAME: &'static str = "Chains of Prometheus";
    pub const DESCRIPTION: &'static str =
        "Whenever a player draws a card, that player taps their strongest untapped minion.";

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
                costs: Costs::mana_only(4),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for ChainsOfPrometheus {}

#[async_trait::async_trait]
impl Card for ChainsOfPrometheus {
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
            id: CARD_DRAW_HOOK,
            trigger: EffectQuery::DrawCard { player_id: None },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let Effect::DrawCard { player_id, .. } = effect else {
                    return Ok(vec![]);
                };
                let drawing_player = *player_id;

                // Find the drawing player's strongest untapped minion.
                let untapped_minions = CardQuery::new()
                    .minions()
                    .untapped()
                    .controlled_by(&drawing_player)
                    .all(state);

                if untapped_minions.is_empty() {
                    return Ok(vec![]);
                }

                let max_power = untapped_minions
                    .iter()
                    .filter_map(|id| {
                        let card = state.get_card(id);
                        let power = card.get_power(state).ok()??;
                        Some(power)
                    })
                    .max()
                    .unwrap_or_default();
                let strongest = untapped_minions
                    .into_iter()
                    .filter(|id| {
                        let card = state.get_card(id);
                        match card.get_power(state) {
                            Err(_) => false,
                            Ok(power) => power.unwrap_or_default() == max_power,
                        }
                    })
                    .collect::<Vec<CardId>>();

                let mut minion_id = strongest[0];
                if strongest.len() > 1 {
                    let Some(picked_card) = CardQuery::from_ids(strongest)
                        .count(1)
                        .pick(&drawing_player, state)
                        .await?
                    else {
                        return Ok(vec![]);
                    };

                    minion_id = picked_card;
                }

                Ok(vec![Effect::SetTapped {
                    card_id: minion_id,
                    tapped: true,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ChainsOfPrometheus::NAME, |owner_id: PlayerId| {
        Box::new(ChainsOfPrometheus::new(owner_id))
    });
