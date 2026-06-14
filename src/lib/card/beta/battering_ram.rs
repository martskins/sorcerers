use crate::prelude::*;

#[derive(Debug, Clone)]
struct RamStrike;

impl RamStrike {
    fn valid_targets(&self, card_id: &CardId, state: &State) -> Vec<CardId> {
        let card = state.get_card(card_id);
        let walls = CardQuery::new()
            .auras()
            .adjacent_to(card.get_location())
            .name_contains("Wall ".to_string())
            .all(state);
        let monuments = CardQuery::new()
            .artifacts()
            .adjacent_to(card.get_location())
            .artifact_type(ArtifactType::Monument)
            .all(state);

        let mut targets = vec![];
        targets.extend(walls);
        targets.extend(monuments);
        targets
    }
}

#[async_trait::async_trait]
impl ActivatedAbility for RamStrike {
    fn get_name(&self) -> String {
        "Tap to destroy an adjacent site".to_string()
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let targets = self.valid_targets(card_id, state);
        Ok(!targets.is_empty())
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let targets = self.valid_targets(card_id, state);
        if targets.is_empty() {
            return Ok(vec![]);
        }

        let picked = pick_card(
            player_id,
            &targets,
            state,
            "Battering Ram: Pick a wall or monument to destroy",
        )
        .await?;
        Ok(vec![Effect::BuryCard { card_id: picked }])
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }
}

#[derive(Debug, Clone)]
pub struct BatteringRam {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl BatteringRam {
    pub const NAME: &'static str = "Battering Ram";
    pub const DESCRIPTION: &'static str =
        "Units here have “Tap → Destroy target adjacent Wall or Monument.”";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Device],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(2),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for BatteringRam {}

#[async_trait::async_trait]
impl Card for BatteringRam {
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

    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        Ok(vec![OngoingEffect::GrantActivatedAbility {
            ability: Box::new(RamStrike),
            affected_cards: CardQuery::new()
                .units()
                .in_location(self.get_location().clone()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BatteringRam::NAME, |owner_id: PlayerId| {
    Box::new(BatteringRam::new(owner_id))
});
