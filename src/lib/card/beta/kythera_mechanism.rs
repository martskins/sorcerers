use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct KytheraMechanism {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl KytheraMechanism {
    pub const NAME: &'static str = "Kythera Mechanism";
    pub const DESCRIPTION: &'static str = "Bearer's controller determines all random outcomes.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
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

    async fn card_query_override(
        &self,
        state: &State,
        query: &CardQuery,
    ) -> anyhow::Result<Option<CardQuery>> {
        if !query.is_randomised() {
            return Ok(None);
        }

        let query = query.clone();
        let options = query.all(state);
        Ok(Some(CardQuery::from_ids(options).with_prompt(
            "Kythera Mechanism: Choose a card to override random decision",
        )))
    }

    fn zone_query_override(
        &self,
        state: &State,
        query: &ZoneQuery,
    ) -> anyhow::Result<Option<ZoneQuery>> {
        if !query.is_randomised() {
            return Ok(None);
        }

        let options = query.options(state);
        Ok(Some(ZoneQuery::from_options(
            options,
            Some("Kythera Mechanism: Choose a zone to override random decision".to_string()),
        )))
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (KytheraMechanism::NAME, |owner_id: PlayerId| {
        Box::new(KytheraMechanism::new(owner_id))
    });
