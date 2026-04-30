use crate::{
    card::{AdditionalCost, Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{ActivatedAbility, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct SpearStrike;

#[async_trait::async_trait]
impl ActivatedAbility for SpearStrike {
    fn get_name(&self) -> String {
        "Spear Strike".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, state: &State) -> anyhow::Result<Cost> {
        let bearer_id = state.get_card(card_id).get_bearer_id()?.unwrap_or(*card_id);
        Ok(Cost::additional_only(AdditionalCost::tap(&bearer_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let spear_card = state.get_card(card_id);
        let controller_id = spear_card.get_controller_id(state);
        let target_id = match CardQuery::new()
            .minions()
            .in_play()
            .with_prompt("Spear of Destiny: Choose an enemy minion to destroy")
            .pick(player_id, state, false)
            .await?
        {
            Some(id) => {
                let target = state.get_card(&id);
                if target.get_controller_id(state) == controller_id {
                    return Ok(vec![]);
                }
                id
            }
            None => return Ok(vec![]),
        };
        let target_zone = state.get_card(&target_id).get_zone().clone();
        Ok(vec![
            Effect::BuryCard { card_id: target_id },
            Effect::TeleportCard {
                player_id: controller_id,
                card_id: *card_id,
                to_zone: target_zone,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct SpearOfDestiny {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl SpearOfDestiny {
    pub const NAME: &'static str = "Spear of Destiny";
    pub const DESCRIPTION: &'static str = "Tap bearer → destroy target minion and move Spear of Destiny to its location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: true,
                types: vec![ArtifactType::Weapon],
                tapped: false,
                region: Region::Surface,
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(5),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for SpearOfDestiny {}

#[async_trait::async_trait]
impl Card for SpearOfDestiny {
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
        Ok(vec![Box::new(SpearStrike)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SpearOfDestiny::NAME, |owner_id: PlayerId| {
    Box::new(SpearOfDestiny::new(owner_id))
});
