use crate::prelude::*;

const STRIKE_OCCUPANTS_HOOK: HookId = 1;

const CARDINAL_DIRECTIONS: &[Direction] = &[
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

#[derive(Debug, Clone)]
struct TapMoveAndStrike;

#[async_trait::async_trait]
impl ActivatedAbility for TapMoveAndStrike {
    fn get_name(&self) -> String {
        "Tap → Move three steps, striking each untapped unit along the way".to_string()
    }

    async fn on_select(
        &self,
        card_id: &CardId,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let bull_demons = state.get_card(card_id);
        let start_zone = bull_demons.get_location().clone();

        let direction = pick_direction(
            player_id,
            CARDINAL_DIRECTIONS,
            state,
            "Bull Demons of Adum: Choose a cardinal direction to charge",
            *card_id,
        )
        .await?;

        let mut effects: Vec<Effect> = vec![];
        let mut path: Vec<Location> = vec![];
        let mut current_location = start_zone.clone();

        for _ in 0..3 {
            match current_location.steps_in_direction(&direction, 1, state, Some(card_id)) {
                Some(next_location) => {
                    // Strike each untapped unit in the destination zone.
                    let targets = CardQuery::new()
                        .units()
                        .untapped()
                        .id_not(*card_id)
                        .in_location(next_location.clone())
                        .all(state);

                    for target_id in targets {
                        effects.push(Effect::Strike {
                            striker_id: *card_id,
                            target_id,
                        });
                    }

                    path.push(next_location.clone());
                    current_location = next_location;
                }
                None => break,
            }
        }

        if let Some(final_zone) = path.last() {
            effects.push(Effect::MoveCard {
                player_id: *player_id,
                card_id: *card_id,
                from: start_zone,
                to: LocationQuery::from_location(
                    final_zone.with_region(state.get_card(card_id).get_region(state).clone()),
                ),
                tap: false,
                through_path: Some(path),
            });
        }

        Ok(effects)
    }

    fn get_cost(&self, card_id: &CardId, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }
}

#[derive(Debug, Clone)]
pub struct BullDemonsOfAdum {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl BullDemonsOfAdum {
    pub const NAME: &'static str = "Bull Demons of Adum";
    pub const DESCRIPTION: &'static str = "Tap → Move three steps in a cardinal direction. When Bull Demons of Adum enter each location, they strike each untapped unit there.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![],
                types: vec![MinionType::Demon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
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
impl Card for BullDemonsOfAdum {
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
        _state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        Ok(vec![Box::new(TapMoveAndStrike)])
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: STRIKE_OCCUPANTS_HOOK,
            trigger: EffectQuery::EnterLocation {
                card: self.get_id().into(),
                location: LocationQuery::new(),
                from: None,
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
            STRIKE_OCCUPANTS_HOOK => Ok(CardQuery::new()
                .units()
                .untapped()
                .in_zone(self.get_zone())
                .all(state)
                .into_iter()
                .map(|target_id| Effect::Strike {
                    striker_id: *self.get_id(),
                    target_id,
                })
                .collect()),
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BullDemonsOfAdum::NAME, |owner_id: PlayerId| {
        Box::new(BullDemonsOfAdum::new(owner_id))
    });
