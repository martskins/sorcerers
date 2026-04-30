use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs,
        Edition, Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct IronShackles {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl IronShackles {
    pub const NAME: &'static str = "Iron Shackles";
    pub const DESCRIPTION: &'static str = "Conjure onto an enemy minion. Bearer is Disabled.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Relic],
                tapped: false,
                region: Region::Surface,
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

impl Artifact for IronShackles {
    fn get_valid_attach_targets(&self, state: &State) -> Vec<uuid::Uuid> {
        let controller_id = self.get_controller_id(state);
        CardQuery::new()
            .minions()
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect()
    }
}

#[async_trait::async_trait]
impl Card for IronShackles {
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

    async fn get_continuous_effects(&self, _state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = self
            .get_artifact()
            .expect("IronShackles should have artifact base")
            .get_bearer()?;

        let Some(bearer_id) = bearer_id else {
            return Ok(vec![]);
        };

        Ok(vec![ContinuousEffect::GrantAbility {
            ability: Ability::Disabled,
            affected_cards: bearer_id.into(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (IronShackles::NAME, |owner_id: PlayerId| {
        Box::new(IronShackles::new(owner_id))
    });
