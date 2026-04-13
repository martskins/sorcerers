use std::sync::Arc;

use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct ChainsOfPrometheus {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl ChainsOfPrometheus {
    pub const NAME: &'static str = "Chains of Prometheus";
    pub const DESCRIPTION: &'static str =
        "Whenever a player draws a card, that player taps their strongest untapped minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Monument],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let chains_id = self.get_id().clone();

        let deferred = DeferredEffect {
            trigger_on_effect: EffectQuery::DrawCard { player_id: None },
            expires_on_effect: Some(EffectQuery::BuryCard {
                card: CardQuery::from_id(self.get_id().clone()),
            }),
            on_effect: Arc::new(
                move |state: &State, _card_id: &uuid::Uuid, effect: &Effect| {
                    let _ = chains_id;
                    Box::pin(async move {
                        // Extract the drawing player from the effect.
                        let drawing_player = match effect {
                            Effect::DrawSpell { player_id, .. } => player_id.clone(),
                            Effect::DrawSite { player_id, .. } => player_id.clone(),
                            Effect::DrawCard { player_id, .. } => player_id.clone(),
                            _ => return Ok(vec![]),
                        };

                        // Find the drawing player's strongest untapped minion.
                        let untapped_minions = CardQuery::new()
                            .minions()
                            .untapped()
                            .controlled_by(&drawing_player)
                            .all(state);

                        if untapped_minions.is_empty() {
                            return Ok(vec![]);
                        }

                        // Find the minion with the highest power.
                        let max_power = untapped_minions
                            .iter()
                            .filter_map(|id| {
                                let card = state.get_card(&id);
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
                            .collect::<Vec<uuid::Uuid>>();

                        let picked_card = CardQuery::from_ids(strongest)
                            .count(1)
                            .pick(&drawing_player, state, false)
                            .await?;
                        match picked_card {
                            Some(id) => Ok(vec![Effect::TapCard { card_id: id }]),
                            None => Ok(vec![]),
                        }
                    })
                },
            ),
            multitrigger: true,
        };

        Ok(vec![Effect::AddDeferredEffect { effect: deferred }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (ChainsOfPrometheus::NAME, |owner_id: PlayerId| {
        Box::new(ChainsOfPrometheus::new(owner_id))
    });
