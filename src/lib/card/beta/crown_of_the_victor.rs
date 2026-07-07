use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct CrownOfTheVictor {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl CrownOfTheVictor {
    pub const NAME: &'static str = "Crown of the Victor";
    pub const DESCRIPTION: &'static str = "Bearer has +3 power if they've ever killed a minion.";

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
                costs: Costs::mana_only(3),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for CrownOfTheVictor {}

#[async_trait::async_trait]
impl Card for CrownOfTheVictor {
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

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let Some(bearer_id) = self.get_bearer_id()? else {
            return Ok(vec![]);
        };

        let has_killed = state.effect_log().iter().find(|le| matches!(le.effect, Effect::KillMinion { killer_id, .. } if killer_id == bearer_id));
        if has_killed.is_none() {
            return Ok(vec![]);
        }

        Ok(vec![OngoingEffect::ModifyPower {
            power_diff: 3,
            affected_cards: Box::new(CardQuery::new().bearer_of_card(self.get_id())),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CrownOfTheVictor::NAME, |owner_id: PlayerId| {
        Box::new(CrownOfTheVictor::new(owner_id))
    });
