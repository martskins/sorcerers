use crate::{
    card::{AdditionalCost, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    state::State,
};

#[derive(Debug, Clone)]
struct ReadManuscript;

#[async_trait::async_trait]
impl ActivatedAbility for ReadManuscript {
    fn get_name(&self) -> String {
        "Read Manuscript".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, state: &State) -> anyhow::Result<Cost> {
        let bearer_id = state.get_card(card_id).get_bearer_id()?.unwrap_or(*card_id);
        Ok(Cost::additional_only(AdditionalCost::tap(&bearer_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let manuscript = state.get_card(card_id);
        let controller_id = manuscript.get_controller_id(state);
        let bearer_id = match manuscript.get_bearer_id()? {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        Ok(vec![
            Effect::DrawSpell { player_id: controller_id, count: 1 },
            Effect::TakeDamage {
                card_id: bearer_id,
                from: *card_id,
                damage: 2,
                is_strike: false,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct PnakoticManuscript {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl PnakoticManuscript {
    pub const NAME: &'static str = "Pnakotic Manuscript";
    pub const DESCRIPTION: &'static str = "Tap bearer → DrawSpell. Bearer takes 2 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Relic],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(2),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for PnakoticManuscript {}

#[async_trait::async_trait]
impl Card for PnakoticManuscript {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_artifact_base(&self) -> Option<&ArtifactBase> { Some(&self.artifact_base) }
    fn get_artifact_base_mut(&mut self) -> Option<&mut ArtifactBase> { Some(&mut self.artifact_base) }
    fn get_artifact(&self) -> Option<&dyn Artifact> { Some(self) }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ReadManuscript)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PnakoticManuscript::NAME, |owner_id: PlayerId| {
    Box::new(PnakoticManuscript::new(owner_id))
});
