use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct PendulumOfPeril {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl PendulumOfPeril {
    pub const NAME: &'static str = "Pendulum of Peril";
    pub const DESCRIPTION: &'static str = "At the end of each player's turn, Pendulum of Peril kills all minions at its current location and another adjacent location of that player's choice.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Monument],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(6),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for PendulumOfPeril {}

#[async_trait::async_trait]
impl Card for PendulumOfPeril {
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

    async fn on_turn_end(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        let current_player = state.current_player;
        let adjacent_zones: Vec<Zone> = zone
            .get_adjacent()
            .into_iter()
            .filter(|adjacent| adjacent != zone)
            .collect();
        let chosen_zone = if adjacent_zones.is_empty() {
            None
        } else {
            Some(
                pick_zone(
                    &current_player,
                    &adjacent_zones,
                    state,
                    false,
                    "Pendulum of Peril: Pick an adjacent location to destroy all minions",
                )
                .await?,
            )
        };

        let mut minions = CardQuery::new().minions().in_zone(zone).all(state);
        if let Some(chosen_zone) = chosen_zone {
            minions.extend(CardQuery::new().minions().in_zone(&chosen_zone).all(state));
        }
        minions.sort();
        minions.dedup();

        Ok(minions
            .into_iter()
            .map(|card_id| Effect::KillMinion {
                card_id,
                killer_id: *self.get_id(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PendulumOfPeril::NAME, |owner_id: PlayerId| {
        Box::new(PendulumOfPeril::new(owner_id))
    });
