use crate::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GuileSirens {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GuileSirens {
    pub const NAME: &'static str = "Guile Sirens";
    pub const DESCRIPTION: &'static str = "Submerge\r \r At the start of your turn, force target nearby enemy minion to take a step toward Guile Sirens.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "WW"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

const TURN_START_HOOK: HookId = 1;

#[async_trait::async_trait]
impl Card for GuileSirens {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_START_HOOK,
            trigger: EffectQuery::TurnStart { player_id: None },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            TURN_START_HOOK => {
                if state.current_player() != self.get_controller_id(state) {
                    return Ok(vec![]);
                }

                let controller_id = self.get_controller_id(state);
                let opponent_id = state.get_opponent_id(&controller_id)?;
                let Some(picked_card_id) = CardQuery::new()
                    .controlled_by(&opponent_id)
                    .minions()
                    .adjacent_to(self.get_zone())
                    .with_prompt("Pick a minion to lure in")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state, false)
                    .await?
                else {
                    return Ok(vec![]);
                };
                let picked_card = state.get_card(&picked_card_id);
                let zones = picked_card
                    .get_location()
                    .get_adjacent_locations(state)
                    .into_iter()
                    .map(|location| {
                        let steps = location.steps_to_location(self.get_location());
                        (location, steps)
                    })
                    .collect::<Vec<(Location, Option<u8>)>>();

                let mut steps_to_location = HashMap::new();
                for (location, steps) in zones {
                    if let Some(steps) = steps {
                        steps_to_location
                            .entry(steps)
                            .or_insert(vec![])
                            .push(location.clone());
                    }
                }

                if let Some(min_steps) = steps_to_location.keys().min() {
                    let closest_locations = steps_to_location.get(min_steps).unwrap();
                    let picked_location = if closest_locations.len() == 1 {
                        closest_locations.first().unwrap().clone()
                    } else {
                        pick_location(
                            &opponent_id,
                            closest_locations,
                            state,
                            true,
                            &format!(
                                "Guile Sirens: Pick a location to move {} to",
                                picked_card.get_name()
                            ),
                        )
                        .await?
                    };

                    return Ok(vec![Effect::MoveCard {
                        player_id: opponent_id,
                        card_id: picked_card_id,
                        from: picked_card
                            .get_zone()
                            .clone()
                            .into_location()
                            .expect("Guile Sirens target must be in a location"),
                        to: LocationQuery::from_location(
                            picked_location.with_region(picked_card.get_region(state).clone()),
                        ),
                        tap: false,
                        through_path: None,
                    }]);
                }

                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GuileSirens::NAME, |owner_id: PlayerId| {
    Box::new(GuileSirens::new(owner_id))
});
