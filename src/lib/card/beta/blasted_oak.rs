use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BlastedOak {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl BlastedOak {
    pub const NAME: &'static str = "Blasted Oak";
    pub const DESCRIPTION: &'static str = "If a spell or non-basic ability can target—in order of precedence—Blasted Oak, its site or location, or anything else at its site or location, it must.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                types: vec![ArtifactType::Monument],
                tapped: false,
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

impl Artifact for BlastedOak {}

#[async_trait::async_trait]
impl Card for BlastedOak {
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

    // If a spell or non-basic ability can target—in order of precedence—Blasted Oak,
    // its site or location, or anything else at its site or location, it must.
    // TODO: Revisit this. It's too specific to this card's behavious to be included as part of the
    // Card trait.
    fn restrict_card_query_targets(
        &self,
        state: &State,
        _query: &CardQuery,
        targets: &[CardId],
    ) -> Option<Vec<CardId>> {
        let oak_id = self.get_id();
        let oak_zone = self.get_zone();

        // Precedence 1: Blasted Oak itself
        if targets.contains(oak_id) {
            return Some(vec![*oak_id]);
        }

        // Precedence 2: The site card at Blasted Oak's location
        let site_targets: Vec<CardId> = targets
            .iter()
            .filter(|id| {
                state
                    .cards
                    .values()
                    .any(|c| c.get_id() == *id && c.is_site() && c.get_zone() == oak_zone)
            })
            .cloned()
            .collect();

        if !site_targets.is_empty() {
            return Some(site_targets);
        }

        // Precedence 3: Anything else at Blasted Oak's location
        let at_zone_targets: Vec<CardId> = targets
            .iter()
            .filter(|id| {
                *id != oak_id
                    && state
                        .cards
                        .values()
                        .any(|c| c.get_id() == *id && c.get_zone() == oak_zone)
            })
            .cloned()
            .collect();

        if !at_zone_targets.is_empty() {
            return Some(at_zone_targets);
        }

        None
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BlastedOak::NAME, |owner_id: PlayerId| {
    Box::new(BlastedOak::new(owner_id))
});
