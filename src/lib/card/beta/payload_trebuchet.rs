use crate::{
    card::{AdditionalCost, Artifact, ArtifactBase, Card, CardBase, CardType, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds, pick_zone},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
struct ShootPayload;

#[async_trait::async_trait]
impl ActivatedAbility for ShootPayload {
    fn get_name(&self) -> &str {
        "Shoot Payload"
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let zones = state.get_card(card_id).get_zones_within_steps(state, 3);
        let picked_zone = pick_zone(player_id, &zones, state, "Pick a zone to shoot the payload at").await?;
        let units = picked_zone.get_units(state, None);

        Ok(units
            .iter()
            .map(|unit| Effect::TakeDamage {
                card_id: unit.get_id().clone(),
                damage: 3,
                from: card_id.clone(),
            })
            .collect())
    }

    fn get_cost(&self, card_id: &uuid::Uuid, state: &State) -> anyhow::Result<Cost> {
        let bearer_id = state
            .get_card(card_id)
            .get_artifact_base()
            .and_then(|ab| ab.bearer.clone());
        match bearer_id {
            Some(bearer_id) => {
                let bearer = state.get_card(&bearer_id);
                Ok(Cost {
                    mana: 0,
                    thresholds: Thresholds::new(),
                    additional: vec![
                        AdditionalCost::Tap {
                            card: CardQuery::Specific {
                                id: uuid::Uuid::new_v4(),
                                card_id: bearer_id.clone(),
                            },
                        },
                        AdditionalCost::Tap {
                            card: CardQuery::InZone {
                                id: uuid::Uuid::new_v4(),
                                zone: bearer.get_zone().clone(),
                                card_types: Some(vec![CardType::Minion, CardType::Avatar]),
                                planes: None,
                                owner: Some(bearer.get_controller_id(state).clone()),
                                prompt: Some("Tap an untapped ally here".to_string()),
                                tapped: Some(false),
                            },
                        },
                        AdditionalCost::Discard {
                            card: CardQuery::InZone {
                                id: uuid::Uuid::new_v4(),
                                zone: Zone::Hand,
                                card_types: None,
                                prompt: Some("Discard a card from your hand".to_string()),
                                planes: None,
                                owner: Some(bearer.get_controller_id(state).clone()),
                                tapped: None,
                            },
                        },
                    ],
                })
            }
            None => Ok(Cost::zero()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PayloadTrebuchet {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl PayloadTrebuchet {
    pub const NAME: &'static str = "Payload Trebuchet";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                bearer: None,
                needs_bearer: true,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(5, ""),
                plane: Plane::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for PayloadTrebuchet {}

#[async_trait::async_trait]
impl Card for PayloadTrebuchet {
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

    fn get_activated_abilities(&self, _state: &State) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ShootPayload)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (PayloadTrebuchet::NAME, |owner_id: PlayerId| {
    Box::new(PayloadTrebuchet::new(owner_id))
});