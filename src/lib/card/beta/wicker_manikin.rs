use crate::{
    card::{
        Ability, AreaModifiers, Artifact, ArtifactBase, ArtifactType, Card, CardBase,
        CardConstructor, Costs, Edition, Rarity, Region, Zone,
    },
    game::{Element, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct WickerManikin {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl WickerManikin {
    pub const NAME: &'static str = "Wicker Manikin";
    pub const DESCRIPTION: &'static str =
        "Units here have Spellcaster (Air), Spellcaster (Earth), and Spellcaster (Water).";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Automaton],
                tapped: false,
                region: Region::Surface,
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let units = CardQuery::new().units().in_zone(self.get_zone()).all(state);

        AreaModifiers {
            grants_abilities: units
                .into_iter()
                .map(|id| {
                    (
                        id,
                        vec![
                            Ability::Spellcaster(Some(Element::Air)),
                            Ability::Spellcaster(Some(Element::Earth)),
                            Ability::Spellcaster(Some(Element::Water)),
                        ],
                    )
                })
                .collect(),
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WickerManikin::NAME, |owner_id: PlayerId| {
        Box::new(WickerManikin::new(owner_id))
    });
