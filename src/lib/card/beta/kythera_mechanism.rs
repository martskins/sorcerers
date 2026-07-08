use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct KytheraMechanism {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl KytheraMechanism {
    pub const NAME: &'static str = "Kythera Mechanism";
    pub const DESCRIPTION: &'static str = "Bearer's controller determines all random outcomes.";
    const CARD_PROMPT: &'static str = "Choose a card to override random decision";
    const ZONE_PROMPT: &'static str = "Choose a zone to override random decision";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(1),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for KytheraMechanism {}

#[async_trait::async_trait]
impl Card for KytheraMechanism {
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
        if self.get_bearer()?.is_none() {
            return Ok(vec![]);
        }

        Ok(vec![
            OngoingEffect::choose_from_random_card_options(
                *self.get_id(),
                Self::CARD_PROMPT,
                usize::MAX,
            ),
            OngoingEffect::choose_from_random_zone_options(
                *self.get_id(),
                Self::ZONE_PROMPT,
                usize::MAX,
            ),
        ])

        // let decision_player = state.get_card(&bearer_id).get_controller_id(state);
        // let id = *self.get_id();
        // Ok(vec![
        //     OngoingEffect::ModifyCardQuery {
        //         description: Self::CARD_PROMPT.to_string(),
        //         modifier: Arc::new(move |_state, _player_id, query| {
        //             if !query.is_randomised() {
        //                 return Ok(None);
        //             }
        //
        //             Ok(Some(
        //                 query
        //                     .clone()
        //                     .with_prompt(Self::CARD_PROMPT)
        //                     .with_source_card(id)
        //                     .with_decision_player(decision_player),
        //             ))
        //         }),
        //     },
        //     OngoingEffect::ModifyZoneQuery {
        //         description: Self::ZONE_PROMPT.to_string(),
        //         modifier: Arc::new(move |state, _player_id, query| {
        //             if !query.is_randomised() {
        //                 return Ok(None);
        //             }
        //
        //             let options = query.options(state);
        //             Ok(Some(
        //                 ZoneQuery::from_options(options, Some(Self::ZONE_PROMPT.to_string()))
        //                     .with_source_card(id)
        //                     .with_decision_player(decision_player),
        //             ))
        //         }),
        //     },
        // ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (KytheraMechanism::NAME, |owner_id: PlayerId| {
        Box::new(KytheraMechanism::new(owner_id))
    });
