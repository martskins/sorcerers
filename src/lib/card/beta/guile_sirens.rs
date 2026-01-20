use std::collections::HashMap;

use crate::{
    card::{Ability, Card, CardBase, CardType, Cost, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, pick_card, pick_zone},
    query::ZoneQuery,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct GuileSirens {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl GuileSirens {
    pub const NAME: &'static str = "Guile Sirens";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "WW"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for GuileSirens {
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

        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let minions = CardMatcher {
            controller_id: Some(opponent_id.clone()),
            card_types: Some(vec![CardType::Minion]),
            in_zones: Some(self.get_zone().get_adjacent()),
            ..Default::default()
        }
        .resolve_ids(state);
        let picked_card_id = pick_card(
            &controller_id,
            &minions,
            state,
            "Guile Sirens: Pick a minion to lure in",
        )
        .await?;
        let picked_card = state.get_card(&picked_card_id);

        let zones = picked_card
            .get_zone()
            .get_adjacent()
            .iter()
            .filter(|zone| zone.is_in_play())
            .map(|zone| (zone.clone(), zone.steps_to_zone(self.get_zone())))
            .collect::<Vec<(Zone, Option<usize>)>>();

        let mut steps_to_zone = HashMap::new();
        for (zone, steps) in zones {
            if let Some(steps) = steps {
                steps_to_zone.entry(steps).or_insert(vec![]).push(zone.clone());
            }
        }

        if let Some(min_steps) = steps_to_zone.keys().min() {
            let picked_zone = pick_zone(
                &opponent_id,
                &steps_to_zone.get(min_steps).unwrap(),
                state,
                true,
                &format!("Guile Sirens: Pick a zone to move {} to", picked_card.get_name()),
            )
            .await?;

            return Ok(vec![Effect::MoveCard {
                player_id: opponent_id.clone(),
                card_id: picked_card_id.clone(),
                from: picked_card.get_zone().clone(),
                to: ZoneQuery::from_zone(picked_zone),
                tap: false,
                region: Region::Surface,
                through_path: None,
            }]);
        }

        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (GuileSirens::NAME, |owner_id: PlayerId| {
    Box::new(GuileSirens::new(owner_id))
});
