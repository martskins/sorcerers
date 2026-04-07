use crate::{
    card::{Ability, Artifact, ArtifactBase, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    game::PlayerId,
    state::{CardMatcher, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct AllTerrainVestments {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl AllTerrainVestments {
    pub const NAME: &'static str = "All-Terrain Vestments";

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
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for AllTerrainVestments {}

#[async_trait::async_trait]
impl Card for AllTerrainVestments {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = self
            .get_artifact()
            .expect("All-Terrain Vestments should have an artifact base")
            .get_bearer()?;

        match bearer_id {
            Some(bearer_id) => {
                let bearer = state.get_card(&bearer_id);
                if !bearer.is_minion() {
                    return Ok(vec![]);
                }

                Ok(vec![
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Burrowing,
                        affected_cards: CardMatcher::from_id(bearer_id),
                    },
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Submerge,
                        affected_cards: CardMatcher::from_id(bearer_id),
                    },
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Voidwalk,
                        affected_cards: CardMatcher::from_id(bearer_id),
                    },
                ])
            }
            None => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AllTerrainVestments::NAME, |owner_id: PlayerId| {
        Box::new(AllTerrainVestments::new(owner_id))
    });
