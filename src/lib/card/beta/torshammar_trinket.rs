use crate::{
    card::{
        Artifact, ArtifactBase, ArtifactType, Card, CardBase, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct TorshammarTrinket {
    artifact_base: ArtifactBase,
    card_base: CardBase,
}

impl TorshammarTrinket {
    pub const NAME: &'static str = "Torshammar Trinket";
    pub const DESCRIPTION: &'static str =
        "At the end of your turn, return Torshammar Trinket to your hand.";

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
                costs: Costs::mana_only(1),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Artifact for TorshammarTrinket {}

#[async_trait::async_trait]
impl Card for TorshammarTrinket {
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

        let zone = self.get_zone();
        if !zone.is_in_play() {
            return Ok(vec![]);
        }

        let owner_id = *self.get_owner_id();
        Ok(vec![Effect::MoveCard {
            player_id: owner_id,
            card_id: *self.get_id(),
            from: zone.clone(),
            to: ZoneQuery::from_zone(Zone::Hand),
            tap: false,
            region: Region::Surface,
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TorshammarTrinket::NAME, |owner_id: PlayerId| {
        Box::new(TorshammarTrinket::new(owner_id))
    });
