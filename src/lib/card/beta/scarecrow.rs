use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

/// **Scarecrow** — Ordinary Artifact (Relic, 2 cost, no threshold)
///
/// Genesis → Return each Airborne minion here to its owner's hand.
/// Airborne minions can't enter this location.
/// TODO: Implement prevention of Airborne minions entering this zone.
#[derive(Debug, Clone)]
pub struct Scarecrow {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl Scarecrow {
    pub const NAME: &'static str = "Scarecrow";
    pub const DESCRIPTION: &'static str = "Genesis → Return each Airborne minion here to its owner's hand.\n\nAirborne minions can't enter this location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Relic],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(2),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for Scarecrow {}

#[async_trait::async_trait]
impl Card for Scarecrow {
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

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        use crate::card::Ability;

        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        let airborne_minions = CardQuery::new()
            .minions()
            .in_zone(zone)
            .with_ability(&Ability::Airborne)
            .all(state);

        Ok(airborne_minions
            .into_iter()
            .map(|card_id| Effect::SetCardZone {
                card_id,
                zone: Zone::Hand,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Scarecrow::NAME, |owner_id: PlayerId| {
    Box::new(Scarecrow::new(owner_id))
});
