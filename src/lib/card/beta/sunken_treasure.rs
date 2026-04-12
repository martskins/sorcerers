use crate::{
    card::{Artifact, ArtifactBase, ArtifactType, Card, CardBase, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct SunkenTreasure {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl SunkenTreasure {
    pub const NAME: &'static str = "Sunken Treasure";
    pub const DESCRIPTION: &'static str = "If cast, conjure this under an allied water site of an opponent's choice.\r \r When Sunken Treasure is carried to the surface, its controller sacrifices it and draws two cards.";

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
                costs: Costs::mana_only(1),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for SunkenTreasure {}

#[async_trait::async_trait]
impl Card for SunkenTreasure {
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

    async fn play_mechanic(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;

        let picked_card_id = CardQuery::new()
            .controlled_by(&controller_id)
            .sites()
            .with_prompt("Sunken Treasure: Pick a water site to place the treasure under")
            .with_affinity(Element::Water)
            .pick(&opponent_id, state, false)
            .await?;
        if picked_card_id.is_none() {
            return Ok(vec![]);
        }
        let picked_card_id = picked_card_id.expect("value not to be None");
        let picked_zone = state.get_card(&picked_card_id).get_zone();
        Ok(vec![
            Effect::SetCardRegion {
                card_id: self.get_id().clone(),
                region: Region::Underwater,
                tap: false,
            },
            Effect::PlayCard {
                player_id: controller_id,
                card_id: self.get_id().clone(),
                zone: picked_zone.into(),
            },
        ])
    }

    fn on_region_change(&self, state: &State, from: &Region, to: &Region) -> anyhow::Result<Vec<Effect>> {
        let subsurface = from == &Region::Underwater || from == &Region::Underground;
        let surfaced = to == &Region::Surface;
        let carried = self.get_bearer().unwrap_or_default().is_some();
        let carried_to_surface = subsurface && surfaced && carried;
        if !carried_to_surface {
            return Ok(vec![]);
        }

        Ok(vec![
            Effect::DrawCard {
                player_id: self.get_controller_id(state),
                count: 2,
            },
            Effect::BuryCard {
                card_id: self.get_id().clone(),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (SunkenTreasure::NAME, |owner_id: PlayerId| {
    Box::new(SunkenTreasure::new(owner_id))
});
