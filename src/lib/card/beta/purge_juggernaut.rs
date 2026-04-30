use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct PurgeJuggernaut {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PurgeJuggernaut {
    pub const NAME: &'static str = "Purge Juggernaut";
    pub const DESCRIPTION: &'static str = "At the start of your turn, move to an adjacent site and bury all other minions there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                types: vec![MinionType::Automaton],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::mana_only(6),
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
impl Card for PurgeJuggernaut {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }
        let self_id = *self.get_id();
        let adjacent = self.get_zone().get_adjacent();
        let target_zone = adjacent
            .into_iter()
            .find(|z| z.get_site(state).is_some());
        let target_zone = match target_zone {
            Some(z) => z,
            None => return Ok(vec![]),
        };
        let units_to_bury: Vec<Effect> = CardQuery::new()
            .units()
            .in_zone(&target_zone)
            .id_not(self.get_id())
            .all(state)
            .into_iter()
            .map(|unit_id| Effect::BuryCard { card_id: unit_id })
            .collect();
        let mut effects = vec![
            Effect::TeleportCard { player_id: controller_id, card_id: self_id, to_zone: target_zone },
            Effect::TapCard { card_id: self_id },
        ];
        effects.extend(units_to_bury);
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PurgeJuggernaut::NAME, |owner_id: PlayerId| {
    Box::new(PurgeJuggernaut::new(owner_id))
});
