use crate::{
    card::{Ability, AreaModifiers, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct BeastOfBurden {
    pub card_base: CardBase,
    pub unit_base: UnitBase,
}

impl BeastOfBurden {
    pub const NAME: &'static str = "Beast of Burden";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_mana_and_threshold(2, "F"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
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

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        let controller_id = self.get_controller_id(state);
        let carried_minions = self
            .get_zone()
            .get_minions(state, Some(&controller_id))
            .iter()
            .filter(|minion| minion.get_id() != self.get_id())
            .map(|minion| (minion.get_id().clone(), vec![Ability::Immobile]))
            .collect();

        AreaModifiers {
            grants_abilities: carried_minions,
            ..Default::default()
        }
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
            .unwrap_or_else(|| path.first().cloned().unwrap_or_else(|| self.get_zone().clone()));
        let controller_id = self.get_controller_id(state);

        Ok(state
            .cards
            .iter()
            .filter(|card| card.is_minion())
            .filter(|card| card.get_controller_id(state) == controller_id)
            .filter(|card| card.get_zone() == &from_zone)
            .filter(|card| card.get_id() != self.get_id())
            .map(|card| Effect::MoveCard {
                player_id: controller_id.clone(),
                card_id: card.get_id().clone(),
                from: from_zone.clone(),
                to: to_zone.clone().into(),
                tap: false,
                region: self.get_base().region.clone(),
                through_path: None,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BeastOfBurden::NAME, |owner_id: PlayerId| {
    Box::new(BeastOfBurden::new(owner_id))
});
