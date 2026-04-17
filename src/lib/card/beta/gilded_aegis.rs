use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct GildedAegis {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl GildedAegis {
    pub const NAME: &'static str = "Gilded Aegis";
    pub const DESCRIPTION: &'static str =
        "If bearer is a minion and would die, instead fully heal it and banish Gilded Aegis.";

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
                costs: Costs::mana_only(2),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for GildedAegis {}

#[async_trait::async_trait]
impl Card for GildedAegis {
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

    async fn replace_effect(
        &self,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Option<Vec<Effect>>> {
        let new_effects = match effect {
            Effect::BuryCard { card_id } => {
                let Some(bearer_id) = self.get_bearer_id()? else {
                    return Ok(None);
                };

                if card_id != &bearer_id {
                    return Ok(None);
                }

                let bearer = state.get_card(&bearer_id);
                Some(vec![
                    Effect::BanishCard {
                        card_id: *self.get_id(),
                    },
                    Effect::Heal {
                        card_id: bearer_id,
                        amount: bearer.get_toughness(state).unwrap_or_default(),
                    },
                ])
            }
            _ => None,
        };

        Ok(new_effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GildedAegis::NAME, |owner_id: PlayerId| {
    Box::new(GildedAegis::new(owner_id))
});
