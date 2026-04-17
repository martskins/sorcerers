use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, UnitAction},
    state::State,
};

#[derive(Debug, Clone)]
pub struct BeastOfBurden {
    card_base: CardBase,
    unit_base: UnitBase,
}

impl BeastOfBurden {
    pub const NAME: &'static str = "Beast of Burden";
    pub const DESCRIPTION: &'static str = "May carry any number of allied minions.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BeastOfBurden {
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
        let controller_id = self.get_controller_id(state);
        let can_pick_up = state
            .cards
            .iter()
            .filter(|card| card.is_minion())
            .filter(|card| card.get_controller_id(state) == controller_id)
            .filter(|card| card.get_zone() == self.get_zone())
            .filter(|card| card.get_id() != self.get_id())
            .find(|card| card.get_bearer_id().unwrap_or_default().is_none())
            .is_some();

        let can_drop = state
            .cards
            .iter()
            .filter(|card| card.is_minion())
            .find(|card| card.get_bearer_id().unwrap_or_default() == Some(*self.get_id()))
            .is_some();

        let mut abilities = vec![];
        if can_pick_up {
            abilities.push(Box::new(UnitAction::PickUpMinion) as Box<dyn ActivatedAbility>);
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
            .unwrap_or_else(|| {
                path.first()
                    .cloned()
                    .unwrap_or_else(|| self.get_zone().clone())
            });
        let controller_id = self.get_controller_id(state);

        Ok(state
            .cards
            .iter()
            .filter(|card| card.is_minion())
            .filter(|card| card.get_controller_id(state) == controller_id)
            .filter(|card| card.get_zone() == &from_zone)
            .filter(|card| card.get_bearer_id().unwrap_or_default() == Some(*self.get_id()))
            .filter(|card| card.get_id() != self.get_id())
            .map(|card| Effect::MoveCard {
                player_id: controller_id,
                card_id: *card.get_id(),
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BeastOfBurden::NAME, |owner_id: PlayerId| {
        Box::new(BeastOfBurden::new(owner_id))
    });
