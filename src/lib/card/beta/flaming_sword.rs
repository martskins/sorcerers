use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FlamingSword {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl FlamingSword {
    pub const NAME: &'static str = "Flaming Sword";
    pub const DESCRIPTION: &'static str = "Bearer has +1 power, and its strikes splash full damage to each other enemy at a struck unit's location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Weapon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(4),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for FlamingSword {}

#[async_trait::async_trait]
impl Card for FlamingSword {
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
        Ok(vec![
            OngoingEffect::ModifyPower {
                power_diff: 1,
                affected_cards: Box::new(CardQuery::new().bearer_of_card(self.get_id())),
            },
            OngoingEffect::GrantAbility {
                ability: Ability::SplashDamage,
                affected_cards: Box::new(CardQuery::new().bearer_of_card(self.get_id())),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FlamingSword::NAME, |owner_id: PlayerId| {
    Box::new(FlamingSword::new(owner_id))
});
