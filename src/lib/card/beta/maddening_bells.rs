use crate::{
    card::{
        Ability, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs,
        Edition, Rarity, Region, Zone,
    },
    game::{Element, PlayerId},
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct MaddeningBells {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl MaddeningBells {
    pub const NAME: &'static str = "Maddening Bells";
    pub const DESCRIPTION: &'static str = "Spells by Spellcasters nearby cost 2 more to cast.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Monument, ArtifactType::Instrument],
                tapped: false,
                region: Region::Surface,
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        // Collect all players who have a Spellcaster nearby.
        let spellcaster_player_ids: Vec<PlayerId> = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_zone().is_nearby(self.get_zone()))
            .filter(|c| {
                [
                    Ability::Spellcaster(None),
                    Ability::Spellcaster(Some(Element::Fire)),
                    Ability::Spellcaster(Some(Element::Air)),
                    Ability::Spellcaster(Some(Element::Earth)),
                    Ability::Spellcaster(Some(Element::Water)),
                ]
                .iter()
                .any(|a| c.has_ability(state, a))
            })
            .map(|c| c.get_controller_id(state))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if spellcaster_player_ids.is_empty() {
            return Ok(vec![]);
        }

        // Collect all spell cards (Magic type) for those players in hand or spellbook.
        let affected_spell_ids: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| {
                !c.is_site() && !c.is_avatar() && !c.is_unit() && !c.is_artifact() && !c.is_aura()
            })
            .filter(|c| matches!(c.get_zone(), Zone::Hand | Zone::Spellbook))
            .filter(|c| spellcaster_player_ids.contains(&c.get_controller_id(state)))
            .map(|c| *c.get_id())
            .collect();

        if affected_spell_ids.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![ContinuousEffect::ModifyManaCost {
            mana_diff: 2,
            affected_cards: CardQuery::from_ids(affected_spell_ids),
            zones: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MaddeningBells::NAME, |owner_id: PlayerId| {
        Box::new(MaddeningBells::new(owner_id))
    });
