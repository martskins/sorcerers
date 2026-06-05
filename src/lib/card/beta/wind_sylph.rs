use crate::prelude::*;

const PUSH_UNIT_HOOK: HookId = 1;

const PUSH_DIRECTIONS: [Direction; 8] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
    Direction::TopLeft,
    Direction::TopRight,
    Direction::BottomLeft,
    Direction::BottomRight,
];

#[derive(Debug, Clone)]
pub struct WindSylph {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl WindSylph {
    pub const NAME: &'static str = "Wind Sylph";
    pub const DESCRIPTION: &'static str = "Airborne, Air Spellcaster After Wind Sylph casts a Magic spell, she may push a unit here one step.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne, Ability::Spellcaster(Some(Element::Air))],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
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
impl Card for WindSylph {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: PUSH_UNIT_HOOK,
            trigger: EffectQuery::PlayCard {
                card: CardQuery::new()
                    .card_types(vec![
                        CardType::Minion,
                        CardType::Artifact,
                        CardType::Aura,
                        CardType::Magic,
                    ])
                    .including_not_in_play(),
                spellcaster: Some(self.get_id().into()),
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
            PUSH_UNIT_HOOK => {
                let controller_id = self.get_controller_id(state);
                let units_here = CardQuery::new().units().in_zone(self.get_zone()).all(state);
                if units_here.is_empty()
                    || !yes_or_no_source(
                        &controller_id,
                        state,
                        "Push a unit here one step?",
                        Some(*self.get_id()),
                    )
                    .await?
                {
                    return Ok(vec![]);
                }

                let unit_id = pick_card(
                    &controller_id,
                    &units_here,
                    state,
                    "Wind Sylph: Pick a unit here to push",
                )
                .await?;
                let unit = state.get_card(&unit_id);
                let mut valid_zones = vec![];
                for dir in &PUSH_DIRECTIONS {
                    let Some(zone) = unit.get_zone().zone_in_direction(dir, 1) else {
                        continue;
                    };

                    let can_enter = match zone.get_site(state) {
                        Some(site) => site.can_be_entered_by(
                            &unit_id,
                            unit.get_zone(),
                            unit.get_region(state),
                            state,
                        )?,
                        None => {
                            unit.has_ability(state, &Ability::Voidwalk)
                                && zone.can_be_entered_by(state, &unit_id)?
                        }
                    };

                    if can_enter {
                        valid_zones.push(zone);
                    }
                }
                valid_zones.sort();
                valid_zones.dedup();
                if valid_zones.is_empty() {
                    return Ok(vec![]);
                }

                let target_zone = pick_zone(
                    &controller_id,
                    &valid_zones,
                    state,
                    false,
                    "Wind Sylph: Pick a zone to push that unit to",
                )
                .await?;
                Ok(vec![Effect::MoveCard {
                    player_id: controller_id,
                    card_id: unit_id,
                    from: (unit.get_zone().clone())
                        .into_location()
                        .expect("MoveCard source must be a location"),
                    to: LocationQuery::from_zone(
                        (target_zone).with_region(unit.get_region(state).clone()),
                    ),
                    tap: false,
                    through_path: None,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (WindSylph::NAME, |owner_id: PlayerId| {
    Box::new(WindSylph::new(owner_id))
});
