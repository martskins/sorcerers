use crate::{
    card::{
        Ability, AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_card},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct StealArtifact;

#[async_trait::async_trait]
impl ActivatedAbility for StealArtifact {
    fn get_name(&self) -> String {
        "Steal Artifact".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let controller_id = card.get_controller_id(state);
        let zone = card.get_zone().clone();

        let candidate_units: Vec<uuid::Uuid> = CardQuery::new()
            .units()
            .in_zone(&zone)
            .id_not(card_id)
            .all(state)
            .into_iter()
            .filter(|unit_id| CardQuery::new().artifacts().carried_by(unit_id).any(state))
            .collect();
        if candidate_units.is_empty() {
            return Ok(vec![]);
        }

        let target_unit_id = match CardQuery::from_ids(candidate_units)
            .with_prompt("Sneak Thief: Choose a target unit here")
            .pick(player_id, state, false)
            .await?
        {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let carried_artifacts = CardQuery::new()
            .artifacts()
            .carried_by(&target_unit_id)
            .all(state);
        if carried_artifacts.is_empty() {
            return Ok(vec![]);
        }

        let artifact_id = if carried_artifacts.len() == 1 {
            carried_artifacts[0]
        } else {
            pick_card(
                player_id,
                &carried_artifacts,
                state,
                "Sneak Thief: Choose an artifact to steal",
            )
            .await?
        };

        Ok(vec![
            Effect::SetController {
                card_id: artifact_id,
                player_id: controller_id,
            },
            Effect::SetBearer {
                card_id: artifact_id,
                bearer_id: Some(*card_id),
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct SneakThief {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SneakThief {
    pub const NAME: &'static str = "Sneak Thief";
    pub const DESCRIPTION: &'static str = "Stealth Tap → Steal an artifact out of the hands of another target unit here, and stay Stealthed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Stealth],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "WW"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for SneakThief {
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
    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(StealArtifact)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SneakThief::NAME, |owner_id: PlayerId| {
    Box::new(SneakThief::new(owner_id))
});
