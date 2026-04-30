use crate::{
    card::{
        Ability, AdditionalCost, Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, UnitAction, pick_cards},
    state::State,
};

#[derive(Debug, Clone)]
struct PickUpWeakerMinion;

#[async_trait::async_trait]
impl ActivatedAbility for PickUpWeakerMinion {
    fn get_name(&self) -> String {
        "Pick Up Minion".to_string()
    }

    fn get_cost(
        &self,
        card_id: &uuid::Uuid,
        _state: &State,
    ) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let card = state.get_card(card_id);
        let my_power = match card.get_power(state)? {
            Some(p) => p,
            None => return Ok(vec![]),
        };
        let controller_id = card.get_controller_id(state);

        let weaker_minions: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_controller_id(state) == controller_id)
            .filter(|c| c.get_zone() == card.get_zone())
            .filter(|c| c.get_id() != card_id)
            .filter(|c| c.get_bearer_id().unwrap_or_default().is_none())
            .filter(|c| matches!(c.get_power(state), Ok(Some(p)) if p < my_power))
            .map(|c| *c.get_id())
            .collect();

        let picked = pick_cards(player_id, &weaker_minions, state, "Pick minion to carry").await?;
        Ok(picked
            .into_iter()
            .map(|minion_id| Effect::SetBearer {
                card_id: minion_id,
                bearer_id: Some(*card_id),
            })
            .collect())
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
                region: Region::Surface,
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

        let my_power = match self.get_power(state)? {
            Some(p) => p,
            None => return Ok(vec![]),
        };
        let controller_id = self.get_controller_id(state);

        let can_pick_up = state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_controller_id(state) == controller_id)
            .filter(|c| c.get_zone() == self.get_zone())
            .filter(|c| c.get_id() != self.get_id())
            .filter(|c| c.get_bearer_id().unwrap_or_default().is_none())
            .any(|c| matches!(c.get_power(state), Ok(Some(p)) if p < my_power));

        let can_drop = state
            .cards
            .iter()
            .any(|c| c.get_bearer_id().unwrap_or_default() == Some(*self.get_id()));

        let mut abilities: Vec<Box<dyn ActivatedAbility>> = vec![];
        if can_pick_up {
            abilities.push(Box::new(PickUpWeakerMinion));
        }
        if can_drop {
            abilities.push(Box::new(UnitAction::DropMinion) as Box<dyn ActivatedAbility>);
        }
        Ok(abilities)
    }

    async fn on_move(&self, state: &State, path: &[Zone]) -> anyhow::Result<Vec<Effect>> {
        if path.is_empty() {
            return Ok(vec![]);
        }

        let to_zone = self.get_zone().clone();
        let from_zone = path
            .windows(2)
            .find_map(|pair| {
                if pair[1] == to_zone {
                    Some(pair[0].clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| path.first().cloned().unwrap_or_else(|| to_zone.clone()));

        let controller_id = self.get_controller_id(state);
        let eagle_id = *self.get_id();

        Ok(state
            .cards
            .iter()
            .filter(|c| c.is_minion())
            .filter(|c| c.get_zone() == &from_zone)
            .filter(|c| c.get_bearer_id().unwrap_or_default() == Some(eagle_id))
            .map(|c| Effect::MoveCard {
                player_id: controller_id,
                card_id: *c.get_id(),
                from: from_zone.clone(),
                to: to_zone.clone().into(),
                tap: false,
                region: self.get_region(state).clone(),
                through_path: None,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (HaastEagle::NAME, |owner_id: PlayerId| {
    Box::new(HaastEagle::new(owner_id))
});
