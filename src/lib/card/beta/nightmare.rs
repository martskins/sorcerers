use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Nightmare {
    unit_base: UnitBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl Nightmare {
    pub const NAME: &'static str = "Nightmare";
    pub const DESCRIPTION: &'static str = "At the end of your turn, for each enemy minion here, you may push it to an adjacent location or void.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Undead],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(7, "AA"),
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
impl Card for Nightmare {
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

    fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd {
                player_id: Some(self.get_controller_id(state)),
            },
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
            TURN_END_HOOK => {
                let adjacent_locations = self.get_location().get_adjacent(state);
                if adjacent_locations.is_empty() {
                    return Ok(vec![]);
                }

                let controller_id = self.get_controller_id(state);
                let location = self.get_location().clone();
                let enemy_minions = CardQuery::new()
                    .minions()
                    .not_controlled_by(&controller_id)
                    .in_location(location)
                    .all(state);

                let mut effects = vec![];
                for minion_id in enemy_minions {
                    let minion_loc = state.get_card(&minion_id).get_location().clone();
                    let push = yes_or_no(
                        &controller_id,
                        state,
                        &format!(
                            "Push {} to an adjacent location?",
                            state.get_card(&minion_id).get_name()
                        ),
                        *self.get_id(),
                    )
                    .await?;

                    if !push {
                        continue;
                    }

                    let target_loc = LocationQuery::from_locations(adjacent_locations.clone())
                        .with_prompt("Choose adjacent location to push enemy minion")
                        .with_source_card(*self.get_id())
                        .pick(&controller_id, state)
                        .await?;

                    effects.push(Effect::MoveCard {
                        player_id: controller_id,
                        card_id: minion_id,
                        from: minion_loc,
                        to: target_loc.into(),
                        tap: false,
                        through_path: None,
                    });
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Nightmare::NAME, |owner_id: PlayerId| {
    Box::new(Nightmare::new(owner_id))
});
