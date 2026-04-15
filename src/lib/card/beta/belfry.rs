use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Belfry {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl Belfry {
    pub const NAME: &'static str = "Belfry";
    pub const DESCRIPTION: &'static str = "At the end of your turn, untap all nearby allies.";

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
                costs: Costs::mana_only(3),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for Belfry {}

#[async_trait::async_trait]
impl Card for Belfry {
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
        if self.get_controller_id(state) != state.current_player {
            return Ok(vec![]);
        }

        Ok(CardQuery::new()
            .units()
            .near_to(self.get_zone())
            .all(state)
            .into_iter()
            .filter(|card_id| {
                state.get_card(card_id).get_controller_id(state) == self.get_controller_id(state)
            })
            .map(|card_id| Effect::UntapCard { card_id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Belfry::NAME, |owner_id: PlayerId| {
        Box::new(Belfry::new(owner_id))
    });
