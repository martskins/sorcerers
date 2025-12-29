use rand::seq::IndexedRandom;

use crate::{
    card::{ArtifactBase, Card, CardBase, Edition, Plane, Rarity, Zone},
    game::{PlayerId, Thresholds},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct LuckyCharm {
    pub relic_base: ArtifactBase,
    pub card_base: CardBase,
}

impl LuckyCharm {
    pub const NAME: &'static str = "Lucky Charm";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            relic_base: ArtifactBase { attached_to: None },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 1,
                required_thresholds: Thresholds::parse(""),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for LuckyCharm {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_artifact_base(&self) -> Option<&ArtifactBase> {
        Some(&self.relic_base)
    }

    fn get_relic_base_mut(&mut self) -> Option<&mut ArtifactBase> {
        Some(&mut self.relic_base)
    }

    fn zone_query_override(&self, _state: &State, query: &ZoneQuery) -> Option<ZoneQuery> {
        match query {
            ZoneQuery::Random { options } => {
                let zones = vec![
                    options.choose(&mut rand::rng()).unwrap().clone(),
                    options.choose(&mut rand::rng()).unwrap().clone(),
                ];
                Some(ZoneQuery::FromOptions {
                    options: zones,
                    prompt: Some("Lucky Charm: Choose a zone".to_string()),
                })
            }
            _ => None,
        }
    }

    fn card_query_override(&self, state: &State, query: &CardQuery) -> Option<CardQuery> {
        match query {
            CardQuery::RandomUnitInZone { zone } => {
                let options = zone
                    .get_units(state, None)
                    .iter()
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<_>>();
                let zones = vec![
                    options.choose(&mut rand::rng()).unwrap().clone(),
                    options.choose(&mut rand::rng()).unwrap().clone(),
                ];
                Some(CardQuery::FromOptions {
                    options: zones,
                    prompt: Some("Lucky Charm: Choose a unit".to_string()),
                    preview: true,
                })
            }
            _ => None,
        }
    }
}
