use crate::prelude::*;

#[derive(Debug, Clone)]
struct TapToDealDamage;

#[async_trait::async_trait]
impl ActivatedAbility for TapToDealDamage {
    fn get_name(&self) -> String {
        "Tap to deal 3 damage to target unit".to_string()
    }

    fn get_cost(&self, card_id: &CardId, state: &State) -> anyhow::Result<Cost> {
        let card = state.get_card(card_id);
        let bearer = card
            .get_artifact()
            .ok_or(anyhow::anyhow!("Card is not an artifact"))?
            .get_bearer()?
            .ok_or(anyhow::anyhow!("Artifact has no bearer"))?;
        Ok(Cost::ZERO
            .clone()
            .with_additional(AdditionalCost::tap(bearer))
            .with_additional(AdditionalCost::tap(
                CardQuery::new()
                    .in_zone(card.get_zone())
                    .untapped()
                    .units()
                    .controlled_by(&card.get_controller_id(state)),
            )))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<crate::effect::Effect>> {
        let card = state
            .get_card(card_id)
            .get_artifact()
            .ok_or(anyhow::anyhow!("Card is not an artifact"))?;
        if let Some(bearer_id) = card.get_bearer()? {
            let bearer = state.get_card(&bearer_id);
            let zones = bearer.get_locations_within_steps(state, 2);
            let Some(picked_unit_id) = CardQuery::new()
                .units()
                .in_locations(&zones)
                .with_prompt("Pick a unit to deal 3 damage to")
                .with_source_card(*card_id)
                .pick(&card.get_controller_id(state), state)
                .await?
            else {
                return Ok(vec![]);
            };

            return Ok(vec![Effect::TakeDamage {
                card_id: picked_unit_id,
                from: bearer_id,
                damage: Damage::basic(3),
            }]);
        }

        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct SiegeBallista {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl SiegeBallista {
    pub const NAME: &'static str = "Siege Ballista";
    pub const DESCRIPTION: &'static str =
        "Tap bearer and another ally here -> Deal 3 damage to target unit up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Weapon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
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

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TapToDealDamage)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SiegeBallista::NAME, |owner_id: PlayerId| {
        Box::new(SiegeBallista::new(owner_id))
    });
