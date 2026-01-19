use crate::{
    card::{Ability, Card, CardBase, CardType, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    query::ZoneQuery,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct UnlandAngler {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl UnlandAngler {
    pub const NAME: &'static str = "Unland Angler";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(5, "WW"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for UnlandAngler {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        if state.current_player != self.get_controller_id(state) {
            return Ok(vec![]);
        }

        if self.get_region(state) != &Region::Underwater {
            return Ok(vec![]);
        }

        let controller_id = self.get_controller_id(state);
        let minions = CardMatcher::minions_adjacent(self.get_zone()).resolve_ids(state);
        Ok(minions
            .into_iter()
            .map(|minion_id| {
                let minion = state.get_card(&minion_id);
                Effect::MoveCard {
                    player_id: controller_id.clone(),
                    card_id: minion_id,
                    from: minion.get_zone().clone(),
                    to: ZoneQuery::from_zone(self.get_zone().clone()),
                    tap: minion.is_tapped(),
                    region: minion.get_region(state).clone(),
                    through_path: None,
                }
            })
            .collect::<Vec<Effect>>())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (UnlandAngler::NAME, |owner_id: PlayerId| {
    Box::new(UnlandAngler::new(owner_id))
});
