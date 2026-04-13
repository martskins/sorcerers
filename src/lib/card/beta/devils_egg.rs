use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct DevilSEgg {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl DevilSEgg {
    pub const NAME: &'static str = "Devil's Egg";
    pub const DESCRIPTION: &'static str =
        "At the end of each turn, the controller of Devil's Egg's site loses 1 life.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Relic],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(3),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for DevilSEgg {}

#[async_trait::async_trait]
impl Card for DevilSEgg {
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
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let site = match self.get_zone().get_site(state) {
            Some(s) => s,
            None => return Ok(vec![]),
        };

        let site_controller_id = site.get_controller_id(state);
        let avatar_id = state.get_player_avatar_id(&site_controller_id)?;

        Ok(vec![Effect::TakeDamage {
            card_id: avatar_id,
            from: self.get_id().clone(),
            damage: 1,
            is_strike: false,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (DevilSEgg::NAME, |owner_id: PlayerId| {
        Box::new(DevilSEgg::new(owner_id))
    });
