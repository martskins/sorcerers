use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct QuarrelsomeKobolds {
    unit_base: UnitBase,
    card_base: CardBase,
}

const TURN_END_HOOK: HookId = 1;

impl QuarrelsomeKobolds {
    pub const NAME: &'static str = "Quarrelsome Kobolds";
    pub const DESCRIPTION: &'static str = "At the end of your turn, Quarrelsome Kobolds strike themselves or another target adjacent unit.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![],
                types: vec![MinionType::Goblin],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
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
impl Card for QuarrelsomeKobolds {
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
            id: TURN_END_HOOK,
            trigger: EffectQuery::TurnEnd { player_id: None },
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
                if state.current_player() != self.get_controller_id(state) {
                    return Ok(vec![]);
                }

                let adjacent_locations = self.get_location().get_adjacent(state);
                let mut units = vec![];
                let player_id = self.get_controller_id(state);
                for location in adjacent_locations {
                    let units_in_zone = CardQuery::new()
                        .units()
                        .in_location(location)
                        .can_be_targeted_by_player(&player_id)
                        .all(state);
                    units.extend(units_in_zone);
                }

                let prompt = "Pick a unit to deal damage to";
                let Some(picked_unit) = CardQuery::from_ids(units)
                    .with_prompt(prompt)
                    .with_source_card(*self.get_id())
                    .pick(&self.get_controller_id(state), state)
                    .await?
                else {
                    return Ok(vec![]);
                };
                Ok(vec![Effect::TakeDamage {
                    card_id: picked_unit,
                    from: *self.get_id(),
                    damage: Damage::basic(self.get_power(state)?.unwrap_or(0)),
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (QuarrelsomeKobolds::NAME, |owner_id: PlayerId| {
        Box::new(QuarrelsomeKobolds::new(owner_id))
    });
