use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MaskOfMayhem {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MaskOfMayhem {
    pub const NAME: &'static str = "Mask of Mayhem";
    pub const DESCRIPTION: &'static str = "Whenever a nearby minion can attack, it must.\r \r Nearby strikes against units deal double damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Armor],
                tapped: false,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for MaskOfMayhem {}

#[async_trait::async_trait]
impl Card for MaskOfMayhem {
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

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![ContinuousEffect::DoubleDamageTaken {
            affected_cards: CardQuery::new()
                .units()
                .nearby_locations_to_card(self.get_id()),
            except_strikes: false,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MaskOfMayhem::NAME, |owner_id: PlayerId| {
    Box::new(MaskOfMayhem::new(owner_id))
});
