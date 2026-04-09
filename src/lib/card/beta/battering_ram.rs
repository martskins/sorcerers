use crate::{
    card::{
        AreaModifiers, Artifact, ArtifactBase, ArtifactType, Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_card},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
struct RamStrike;

#[async_trait::async_trait]
impl ActivatedAbility for RamStrike {
    fn get_name(&self) -> String {
        "Tap to destroy an adjacent site".to_string()
    }

    fn can_activate(&self, card_id: &uuid::Uuid, _player_id: &PlayerId, state: &State) -> anyhow::Result<bool> {
        let card = state.get_card(card_id);
        Ok(state.cards.iter().any(|c| {
            c.is_site()
                && c.get_zone() != card.get_zone()
                && c.get_zone().is_adjacent(card.get_zone())
                && c.can_be_targetted_by(state, &card.get_controller_id(state))
        }))
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::from_additional(crate::card::AdditionalCost::Tap {
            card: CardQuery::from_id(card_id.clone()),
        }))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let targets: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_site())
            .filter(|c| c.get_zone() != card.get_zone())
            .filter(|c| c.get_zone().is_adjacent(card.get_zone()))
            .filter(|c| c.can_be_targetted_by(state, player_id))
            .map(|c| c.get_id().clone())
            .collect();
        if targets.is_empty() {
            return Ok(vec![]);
        }

        let picked = pick_card(player_id, &targets, state, "Battering Ram: Pick a site to destroy").await?;
        Ok(vec![Effect::BuryCard { card_id: picked }])
    }
}

#[derive(Debug, Clone)]
pub struct BatteringRam {
    pub artifact_base: ArtifactBase,
    pub card_base: CardBase,
}

impl BatteringRam {
    pub const NAME: &'static str = "Battering Ram";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            artifact_base: ArtifactBase {
                needs_bearer: false,
                types: vec![ArtifactType::Device],
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana(2),
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

impl Artifact for BatteringRam {}

#[async_trait::async_trait]
impl Card for BatteringRam {
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let abilities = self
            .get_zone()
            .get_units(state, None)
            .iter()
            .map(|unit| {
                (
                    unit.get_id().clone(),
                    vec![Box::new(RamStrike) as Box<dyn ActivatedAbility>],
                )
            })
            .collect();

        AreaModifiers {
            grants_activated_abilities: abilities,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BatteringRam::NAME, |owner_id: PlayerId| {
    Box::new(BatteringRam::new(owner_id))
});
