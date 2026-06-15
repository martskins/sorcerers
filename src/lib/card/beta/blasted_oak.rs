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

    fn applies_to_source(state: &State, oak_id: CardId, source_card_id: Option<CardId>) -> bool {
        let Some(source_card_id) = source_card_id else {
            return false;
        };

        state
            .try_get_card(&source_card_id)
            .is_some_and(|source| source.is_magic() || source_card_id != oak_id)
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

    // TODO: Review this implementation
    async fn get_ongoing_effects(&self, _state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let oak_id = *self.get_id();

        Ok(vec![
            OngoingEffect::RestrictCardTargets {
                description: "Blasted Oak must be targeted".to_string(),
                restriction: std::sync::Arc::new(move |state, _player_id, query, targets| {
                    if !Self::applies_to_source(state, oak_id, query.source_card_id()) {
                        return None;
                    }

                    let oak_zone = state.get_card(&oak_id).get_zone();

                    if targets.contains(&oak_id) {
                        return Some(vec![oak_id]);
                    }

                    let site_targets: Vec<CardId> = targets
                        .iter()
                        .filter(|&&id| {
                            let card = state.get_card(&id);
                            card.is_site() && card.get_zone() == oak_zone
                        })
                        .cloned()
                        .collect();

                    if !site_targets.is_empty() {
                        return Some(site_targets);
                    }

                    let at_zone_targets: Vec<CardId> = targets
                        .iter()
                        .filter(|&&id| id != oak_id && state.get_card(&id).get_zone() == oak_zone)
                        .cloned()
                        .collect();

                    if !at_zone_targets.is_empty() {
                        return Some(at_zone_targets);
                    }

                    None
                }),
            },
            OngoingEffect::ModifyZoneQuery {
                description: "Blasted Oak must be targeted by location".to_string(),
                modifier: std::sync::Arc::new(move |state, _player_id, query| {
                    if !Self::applies_to_source(state, oak_id, query.source_card_id()) {
                        return Ok(None);
                    }

                    let oak_zone = state.get_card(&oak_id).get_zone();
                    if query.options(state).contains(oak_zone) {
                        return Ok(Some(
                            ZoneQuery::from_options(
                                vec![oak_zone.clone()],
                                Some("Blasted Oak: choose its location".to_string()),
                            )
                            .with_source_card(oak_id),
                        ));
                    }

                    Ok(None)
                }),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BlastedOak::NAME, |owner_id: PlayerId| {
    Box::new(BlastedOak::new(owner_id))
});
