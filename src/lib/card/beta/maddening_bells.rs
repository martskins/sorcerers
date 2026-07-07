use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MaddeningBells {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MaddeningBells {
    pub const NAME: &'static str = "Maddening Bells";
    pub const DESCRIPTION: &'static str =
        "Spells cast by a nearby Spellcaster cost ② more to cast.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Relic],
                tapped: false,
                ..Default::default()
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

impl Artifact for MaddeningBells {}

#[async_trait::async_trait]
impl Card for MaddeningBells {
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

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let all_spellcaster_abilities = vec![
            Ability::Spellcaster(None),
            Ability::Spellcaster(Some(Element::Fire)),
            Ability::Spellcaster(Some(Element::Air)),
            Ability::Spellcaster(Some(Element::Earth)),
            Ability::Spellcaster(Some(Element::Water)),
        ];
        // Collect all players who have a Spellcaster nearby.
        let spellcasters = CardQuery::new()
            .units()
            .near_to(self.get_location())
            .with_any_ability(all_spellcaster_abilities)
            .all(state);

        // TODO: Missing spellcaster filter
        Ok(vec![OngoingEffect::ModifyManaCost {
            mana_diff: 2,
            affected_cards: Box::new(CardQuery::new().magics()),
            zones: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MaddeningBells::NAME, |owner_id: PlayerId| {
        Box::new(MaddeningBells::new(owner_id))
    });
