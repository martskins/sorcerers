use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Telekinesis {
    card_base: CardBase,
}

impl Telekinesis {
    pub const NAME: &'static str = "Telekinesis";
    pub const DESCRIPTION: &'static str =
        "Caster snatches and picks up target nearby artifact they can carry.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Telekinesis {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_zone = caster.get_zone().clone();
        let caster_region = caster.get_region(state).clone();

        let carryable_artifacts: Vec<uuid::Uuid> = crate::state::CardQuery::new()
            .artifacts()
            .near_to(&caster_zone)
            .in_region(&caster_region)
            .all(state)
            .into_iter()
            .filter(|artifact_id| {
                let artifact = state.get_card(artifact_id);
                artifact.get_bearer_id().unwrap_or_default().is_none()
                    && artifact
                        .get_artifact()
                        .is_some_and(|a| a.get_valid_attach_targets(state).contains(caster_id))
            })
            .collect();

        if carryable_artifacts.is_empty() {
            return Ok(vec![]);
        }

        let artifact_id = pick_card(
            &controller_id,
            &carryable_artifacts,
            state,
            "Telekinesis: Pick a nearby artifact to carry",
        )
        .await?;

        Ok(vec![Effect::SetBearer {
            card_id: artifact_id,
            bearer_id: Some(*caster_id),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Telekinesis::NAME, |owner_id: PlayerId| {
    Box::new(Telekinesis::new(owner_id))
});
