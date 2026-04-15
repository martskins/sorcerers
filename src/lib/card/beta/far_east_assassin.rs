use crate::{
    card::{
        Ability, AdditionalCost, Card, CardBase, Cost, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, pick_card},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
struct ThrowArtifactAbility;

#[async_trait::async_trait]
impl ActivatedAbility for ThrowArtifactAbility {
    fn get_name(&self) -> String {
        "Throw Artifact".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &uuid::Uuid,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let carried = state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter(|c| c.get_bearer_id().ok().flatten().as_ref() == Some(card_id))
            .count();
        Ok(carried > 0)
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);

        let carried: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_artifact())
            .filter(|c| c.get_bearer_id().ok().flatten().as_ref() == Some(card_id))
            .map(|c| c.get_id().clone())
            .collect();

        let artifact_id = pick_card(
            player_id,
            &carried,
            state,
            "Far East Assassin: Pick an artifact to throw",
        )
        .await?;

        let artifact = state.get_card(&artifact_id);
        let damage = artifact.get_base().costs.mana_value() as u16;

        let Some(target_id) = CardQuery::new()
            .minions()
            .adjacent_to(card.get_zone())
            .with_prompt("Far East Assassin: Pick a target unit in an adjacent zone")
            .pick(player_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let target = state.get_card(&target_id);
        Ok(vec![
            Effect::TakeDamage {
                card_id: target_id,
                from: card_id.clone(),
                damage,
                is_strike: false,
            },
            Effect::MoveCard {
                player_id: player_id.clone(),
                card_id: artifact_id,
                from: artifact.get_zone().clone(),
                to: ZoneQuery::from_zone(target.get_zone().clone()),
                tap: false,
                region: target.get_region(state).clone(),
                through_path: None,
            },
            Effect::SetBearer {
                card_id: artifact_id.clone(),
                bearer_id: None,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct FarEastAssassin {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl FarEastAssassin {
    pub const NAME: &'static str = "Far East Assassin";
    pub const DESCRIPTION: &'static str = "Stealth\r \r Tap → Far East Assassin throws an artifact he carries at target adjacent unit. It takes damage equal to the artifact's mana cost.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Stealth],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Card for FarEastAssassin {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_additional_activated_abilities(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(ThrowArtifactAbility)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (FarEastAssassin::NAME, |owner_id: PlayerId| {
        Box::new(FarEastAssassin::new(owner_id))
    });
