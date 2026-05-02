use crate::{
    card::{
        AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity,
        Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
struct PossessArtifact;

#[async_trait::async_trait]
impl ActivatedAbility for PossessArtifact {
    fn get_name(&self) -> String {
        "Tap → Control nearby artifact".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        let nearby_artifacts = CardQuery::new()
            .artifacts()
            .near_to(card.get_zone())
            .all(state);
        Ok(!nearby_artifacts.is_empty())
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let nearby_artifacts = CardQuery::new()
            .artifacts()
            .near_to(card.get_zone())
            .all(state);
        if nearby_artifacts.is_empty() {
            return Ok(vec![]);
        }

        let picked_artifact_id = crate::game::pick_card(
            player_id,
            &nearby_artifacts,
            state,
            "Grösse Poltergeist: Pick a nearby artifact to possess",
        )
        .await?;

        Ok(vec![Effect::SetCardData {
            card_id: *card_id,
            data: Box::new(Some(picked_artifact_id)),
        }])
    }
}

#[derive(Debug, Clone)]
pub struct GrossePoltergeist {
    unit_base: UnitBase,
    card_base: CardBase,
    controlled_artifact: Option<uuid::Uuid>,
}

impl GrossePoltergeist {
    pub const NAME: &'static str = "Grösse Poltergeist";
    pub const DESCRIPTION: &'static str = "Tap → Until Grosse Poltergeist leaves the realm, gain control of a nearby artifact and animate it. It's an Automaton with power equal to its cost, and has its own bearer abilities.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            controlled_artifact: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for GrossePoltergeist {
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

    fn set_data(&mut self, data: &Box<dyn std::any::Any + Send + Sync>) -> anyhow::Result<()> {
        if let Some(controlled_artifact) = data.downcast_ref::<Option<uuid::Uuid>>() {
            self.controlled_artifact = *controlled_artifact;
            return Ok(());
        }

        Err(anyhow::anyhow!("Invalid data type for {}", Self::NAME))
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(PossessArtifact)])
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let Some(artifact_id) = self.controlled_artifact else {
            return Ok(vec![]);
        };

        if !state.get_card(&artifact_id).get_zone().is_in_play() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ControllerOverride {
            controller_id: self.get_controller_id(state),
            affected_cards: CardQuery::from_id(artifact_id),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (GrossePoltergeist::NAME, |owner_id: PlayerId| {
        Box::new(GrossePoltergeist::new(owner_id))
    });
