use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct TheImmortalThrone {
    artifact_base: ArtifactBase,
    card_base: CardBase,
    level_counters: u8,
}

impl TheImmortalThrone {
    pub const NAME: &'static str = "The Immortal Throne";
    pub const DESCRIPTION: &'static str = "Whenever anyone plays a card with cost equal to the number of level counters on The Immortal Throne, they draw a card and add a level counter.\r \r At level 8 or more, an Avatar here alone wins the game.";

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
                costs: Costs::mana_only(4),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            level_counters: 0,
        }
    }
}

impl Artifact for TheImmortalThrone {}

#[async_trait::async_trait]
impl Card for TheImmortalThrone {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(level_counters) = data.downcast_ref::<u8>() {
            self.level_counters = *level_counters;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Invalid data type for The Immortal Throne: expected u8"
            ))
        }
    }

    async fn on_effect(&self, state: &State, effect: &Effect) -> anyhow::Result<Vec<Effect>> {
        let winning_effects = self.winning_effects(state);
        if !winning_effects.is_empty() {
            return Ok(winning_effects);
        }

        match effect {
            Effect::PlayCard {
                player_id, card_id, ..
            } => {
                let card = state.get_card(card_id);
                if card.get_costs(state)?.mana_value() != self.level_counters {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::DrawCard {
                        player_id: *player_id,
                        count: 1,
                        kind: DrawKind::Choice,
                    },
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: std::sync::Arc::new(self.level_counters + 1),
                    },
                ])
            }
            Effect::PlayMagic {
                player_id, card_id, ..
            } => {
                let card = state.get_card(card_id);
                if card.get_costs(state)?.mana_value() != self.level_counters {
                    return Ok(vec![]);
                }

                Ok(vec![
                    Effect::DrawCard {
                        player_id: *player_id,
                        count: 1,
                        kind: DrawKind::Choice,
                    },
                    Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: std::sync::Arc::new(self.level_counters + 1),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

impl TheImmortalThrone {
    fn winning_effects(&self, state: &State) -> Vec<Effect> {
        if self.level_counters < 8 || !self.get_zone().is_in_play() {
            return vec![];
        }

        let avatars_here = CardQuery::new()
            .avatars()
            .in_zone(self.get_zone())
            .all(state);
        let other_units_here = CardQuery::new()
            .units()
            .in_zone(self.get_zone())
            .all(state)
            .into_iter()
            .filter(|card_id| !avatars_here.contains(card_id))
            .collect::<Vec<uuid::Uuid>>();

        if avatars_here.len() != 1 || !other_units_here.is_empty() {
            return vec![];
        }

        let winner = state.get_card(&avatars_here[0]).get_controller_id(state);
        state
            .players
            .iter()
            .filter(|player| player.id != winner)
            .map(|player| Effect::PlayerLost {
                player_id: player.id,
            })
            .collect()
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TheImmortalThrone::NAME, |owner_id: PlayerId| {
        Box::new(TheImmortalThrone::new(owner_id))
    });
