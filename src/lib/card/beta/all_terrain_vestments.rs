use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity,
        Region, Zone,
    },
    game::PlayerId,
    state::{ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct AllTerrainVestments {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl AllTerrainVestments {
    pub const NAME: &'static str = "All-Terrain Vestments";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Armor],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
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
            Some(ref bearer_id) => {
                let bearer = state.get_card(bearer_id);
                if !bearer.is_minion() {
                    return Ok(vec![]);
                }

                Ok(vec![
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Burrowing,
                        affected_cards: bearer_id.into(),
                    },
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Submerge,
                        affected_cards: bearer_id.into(),
                    },
                    ContinuousEffect::GrantAbility {
                        ability: Ability::Voidwalk,
                        affected_cards: bearer_id.into(),
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
