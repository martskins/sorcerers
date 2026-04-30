use std::collections::HashMap;

use crate::{
    card::{Ability, AreaModifiers, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
pub struct MagneticMuzzle {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MagneticMuzzle {
    pub const NAME: &'static str = "Magnetic Muzzle";
    pub const DESCRIPTION: &'static str = "Bearer is silenced.";

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
                costs: Costs::mana_only(2),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for MagneticMuzzle {}

#[async_trait::async_trait]
impl Card for MagneticMuzzle {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_artifact_base(&self) -> Option<&ArtifactBase> { Some(&self.artifact_base) }
    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> { Some(&mut self.artifact_base) }
    fn get_artifact(&self) -> Option<&dyn Artifact> { Some(self) }

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let bearer_id = match self.get_bearer_id().ok().flatten() {
            Some(id) => id,
            None => return AreaModifiers::default(),
        };
        let mut removes: HashMap<uuid::Uuid, Vec<Ability>> = HashMap::new();
        removes.insert(
            bearer_id,
            vec![
                Ability::Spellcaster(None),
                Ability::Spellcaster(Some(Element::Fire)),
                Ability::Spellcaster(Some(Element::Water)),
                Ability::Spellcaster(Some(Element::Earth)),
                Ability::Spellcaster(Some(Element::Air)),
            ],
        );
        AreaModifiers {
            removes_abilities: removes,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MagneticMuzzle::NAME, |owner_id: PlayerId| {
    Box::new(MagneticMuzzle::new(owner_id))
});
