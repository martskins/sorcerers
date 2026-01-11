use crate::{
    card::{AdditionalCost, Artifact, ArtifactBase, Card, CardBase, CardType, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId, Thresholds, pick_card},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
struct TapToDealDamage;

#[async_trait::async_trait]
impl ActivatedAbility for TapToDealDamage {
    fn get_name(&self) -> &str {
        "Tap to deal 3 damage to target unit"
    }

    fn get_cost(&self, card_id: &uuid::Uuid, state: &State) -> anyhow::Result<Cost> {
        let card = state.get_card(card_id);
        let bearer = card
            .get_artifact()
            .ok_or(anyhow::anyhow!("Card is not an artifact"))?
            .get_bearer()?
            .ok_or(anyhow::anyhow!("Artifact has no bearer"))?;
        Ok(Cost {
            mana: 0,
            thresholds: Thresholds::new(),
            additional: vec![
                AdditionalCost::Tap {
                    card: CardQuery::Specific {
                        id: uuid::Uuid::new_v4(),
                        card_id: bearer.clone(),
                    },
                },
                AdditionalCost::Tap {
                    card: CardQuery::InZone {
                        id: uuid::Uuid::new_v4(),
                        zone: card.get_zone().clone(),
                        card_types: Some(vec![CardType::Minion, CardType::Avatar]),
                        planes: None,
                        owner: Some(card.get_controller_id(state).clone()),
                        prompt: Some("Tap an untapped ally here".to_string()),
                        tapped: Some(false),
                    },
                },
            ],
        })
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<crate::effect::Effect>> {
        let card = state
            .get_card(card_id)
            .get_artifact()
            .ok_or(anyhow::anyhow!("Card is not an artifact"))?;
        if let Some(bearer_id) = card.get_bearer()? {
            let bearer = state.get_card(&bearer_id);
            let valid_targets: Vec<uuid::Uuid> = bearer
                .get_zones_within_steps(state, 2)
                .iter()
                .flat_map(|z| z.get_units(state, None))
                .filter(|c| c.is_unit())
                .map(|c| c.get_id())
                .cloned()
                .collect();

            let picked_unit_id = pick_card(
                card.get_controller_id(state),
                &valid_targets,
                state,
                "Siege Ballista: Pick a unit to deal 3 damage to",
            )
            .await?;

            return Ok(vec![Effect::TakeDamage {
                card_id: picked_unit_id,
                from: bearer_id,
                damage: 3,
            }]);
        }

        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct SiegeBallista {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl SiegeBallista {
    pub const NAME: &'static str = "Siege Ballista";

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
                cost: Cost::new(3, ""),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

impl Artifact for SiegeBallista {}

#[async_trait::async_trait]
impl Card for SiegeBallista {
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
        Ok(vec![Box::new(TapToDealDamage)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SiegeBallista::NAME, |owner_id: PlayerId| {
    Box::new(SiegeBallista::new(owner_id))
});