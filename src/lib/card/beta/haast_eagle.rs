use crate::prelude::*;

#[derive(Debug, Clone)]
struct PickUpWeakerMinion;

#[async_trait::async_trait]
impl ActivatedAbility for PickUpWeakerMinion {
    fn get_name(&self) -> String {
        "Pick Up Minion".to_string()
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let controller_id = card.get_controller_id(state);
        let Some(picked) = CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_location(card.get_location().clone())
            .power_lt(card.get_power(state)?.unwrap_or_default())
            .with_source_card(*card_id)
            .with_prompt("Pick minion to carry")
            .pick(player_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::SetBearer {
            card_id: picked,
            bearer_id: Some(*card_id),
        }])
    }
}

#[derive(Debug, Clone)]
pub struct HaastEagle {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl HaastEagle {
    pub const NAME: &'static str = "Haast Eagle";
    pub const DESCRIPTION: &'static str = "Airborne\r \r May carry a weaker allied minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "AA"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for HaastEagle {
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
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let can_drop = !CardQuery::new()
            .carried_by(self.get_id())
            .all(state)
            .is_empty();
        let can_pick_up = !CardQuery::new()
            .minions()
            .controlled_by(&controller_id)
            .in_location(self.get_location().clone())
            .power_lt(self.get_power(state)?.unwrap_or_default())
            .all(state)
            .is_empty();

        let mut abilities: Vec<Box<dyn ActivatedAbility>> = vec![];
        if can_pick_up {
            abilities.push(Box::new(PickUpWeakerMinion));
        }
        if can_drop {
            abilities.push(Box::new(UnitAction::DropMinion) as Box<dyn ActivatedAbility>);
        }
        Ok(abilities)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (HaastEagle::NAME, |owner_id: PlayerId| {
    Box::new(HaastEagle::new(owner_id))
});
