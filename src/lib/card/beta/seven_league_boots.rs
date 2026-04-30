use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs,
        Edition, Rarity, Region, Zone,
    },
    game::PlayerId,
    state::{ContinuousEffect, State},
};

/// **Seven-League Boots** — Unique Artifact (Armor, 3 cost)
///
/// Bearer has Movement +7.
#[derive(Debug, Clone)]
pub struct SevenLeagueBoots {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl SevenLeagueBoots {
    pub const NAME: &'static str = "Seven-League Boots";
    pub const DESCRIPTION: &'static str = "Bearer has Movement +7.";

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
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for SevenLeagueBoots {}

#[async_trait::async_trait]
impl Card for SevenLeagueBoots {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let bearer_id = self
            .get_artifact()
            .expect("SevenLeagueBoots should have an artifact base")
            .get_bearer()?;

        match bearer_id {
            Some(ref bearer_id) => {
                let bearer = state.get_card(bearer_id);
                if !bearer.is_minion() {
                    return Ok(vec![]);
                }

                Ok(vec![ContinuousEffect::GrantAbility {
                    ability: Ability::Movement(7),
                    affected_cards: bearer_id.into(),
                }])
            }
            None => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SevenLeagueBoots::NAME, |owner_id: PlayerId| {
        Box::new(SevenLeagueBoots::new(owner_id))
    });
