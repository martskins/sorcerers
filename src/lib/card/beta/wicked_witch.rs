use std::collections::HashMap;

use crate::{
    card::{Ability, AreaModifiers, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::{Counter, Effect},
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct WickedWitch {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl WickedWitch {
    pub const NAME: &'static str = "Wicked Witch";
    pub const DESCRIPTION: &'static str = "Spellcaster. Other nearby minions have -2 power.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WickedWitch {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }

    fn area_modifiers(&self, state: &State) -> AreaModifiers {
        if !self.get_zone().is_in_play() {
            return AreaModifiers::default();
        }
        let self_id = *self.get_id();
        let nearby_zones = self.get_zone().get_nearby();
        let counters: HashMap<uuid::Uuid, Vec<Counter>> = state
            .cards
            .iter()
            .filter(|c| *c.get_id() != self_id)
            .filter(|c| c.get_unit_base().is_some())
            .filter(|c| nearby_zones.contains(c.get_zone()))
            .map(|c| {
                (
                    *c.get_id(),
                    vec![Counter::new(-2, 0, None)],
                )
            })
            .collect();
        AreaModifiers {
            grants_counters: counters,
            ..Default::default()
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WickedWitch::NAME, |owner_id: PlayerId| {
    Box::new(WickedWitch::new(owner_id))
});
