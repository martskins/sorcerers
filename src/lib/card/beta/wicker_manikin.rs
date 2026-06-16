use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WickerManikin {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl WickerManikin {
    pub const NAME: &'static str = "Wicker Manikin";
    pub const DESCRIPTION: &'static str = "Non-fire Spellcaster";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Automaton],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(1),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for WickerManikin {}

#[async_trait::async_trait]
impl Card for WickerManikin {
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
        let affected_cards = CardQuery::new().units().in_zone_of_card(self.get_id());
        Ok(vec![
            OngoingEffect::GrantAbility {
                ability: Ability::Spellcaster(Some(Element::Air)),
                affected_cards: affected_cards.clone(),
            },
            OngoingEffect::GrantAbility {
                ability: Ability::Spellcaster(Some(Element::Earth)),
                affected_cards: affected_cards.clone(),
            },
            OngoingEffect::GrantAbility {
                ability: Ability::Spellcaster(Some(Element::Water)),
                affected_cards,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WickerManikin::NAME, |owner_id: PlayerId| {
        Box::new(WickerManikin::new(owner_id))
    });
