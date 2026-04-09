use crate::{
    card::{Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct AngelsEgg {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl AngelsEgg {
    pub const NAME: &'static str = "Angel's Egg";

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
                costs: Costs::from_mana(3),
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

impl Artifact for AngelsEgg {}

#[async_trait::async_trait]
impl Card for AngelsEgg {
    fn get_name(&self) -> &str {
        Self::NAME
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
        let site = match self.get_zone().get_site(state) {
            Some(site) => site,
            None => return Ok(vec![]),
        };

        let avatar_id = state.get_player_avatar_id(&site.get_controller_id(state))?;
        Ok(vec![Effect::Heal {
            card_id: avatar_id,
            amount: 1,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AngelsEgg::NAME, |owner_id: PlayerId| Box::new(AngelsEgg::new(owner_id)));
