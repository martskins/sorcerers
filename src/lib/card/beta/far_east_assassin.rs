use crate::prelude::*;

#[derive(Debug, Clone)]
struct ThrowArtifactAbility;

#[async_trait::async_trait]
impl ActivatedAbility for ThrowArtifactAbility {
    fn get_name(&self) -> String {
        "Tap -> Throw Artifact".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    fn can_activate(
        &self,
        card_id: &CardId,
        _player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<bool> {
        let carried = CardQuery::new().artifacts().carried_by(card_id).all(state);
        Ok(!carried.is_empty())
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let Some(artifact_id) = CardQuery::new()
            .artifacts()
            .carried_by(card_id)
            .with_source_card(*card_id)
            .with_prompt("Pick an artifact to throw")
            .pick(player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let artifact = state.get_card(&artifact_id);
        let damage = artifact
            .get_base()
            .costs
            .printed_mana_value()
            .unwrap_or_default() as u16;

        let Some(target_id) = CardQuery::new()
            .units()
            .adjacent_to(card.get_location())
            .with_source_card(*card_id)
            .with_prompt("Pick a target unit in an adjacent zone")
            .pick(player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        let target = state.get_card(&target_id);
        Ok(vec![
            Effect::TakeDamage {
                card_id: target_id,
                from: *card_id,
                damage: Damage::basic(damage),
            },
            Effect::MoveCard {
                player_id: *player_id,
                card_id: artifact_id,
                from: artifact.get_location().clone(),
                to: target.get_location().clone().into(),
                tap: false,
                through_path: None,
            },
            Effect::SetBearer {
                card_id: artifact_id,
                bearer_id: None,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct FarEastAssassin {
    unit_base: UnitBase,
    card_base: CardBase,
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
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (FarEastAssassin::NAME, |owner_id: PlayerId| {
        Box::new(FarEastAssassin::new(owner_id))
    });
