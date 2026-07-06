use crate::prelude::*;

const RANGED_STRIKE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct SkirmishersOfMu {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SkirmishersOfMu {
    pub const NAME: &'static str = "Skirmishers of Mu";
    pub const DESCRIPTION: &'static str = "Ranged\r \r During basic movement, Skirmishers of Mu may perform a ranged strike from any location along their path.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Ranged(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
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
impl Card for SkirmishersOfMu {
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
            id: RANGED_STRIKE_HOOK,
            trigger: EffectQuery::MoveCard {
                card: self.get_id().into(),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            RANGED_STRIKE_HOOK => {
                let Effect::MoveCard {
                    player_id,
                    from,
                    to,
                    through_path,
                    ..
                } = effect
                else {
                    return Ok(vec![]);
                };

                let controller_id = self.get_controller_id(state);
                let to = to.pick(player_id, state).await?;
                let mut path = vec![from.clone(), to];
                if let Some(through_path) = through_path {
                    path = through_path.to_vec();
                }

                let strike =
                    yes_or_no(controller_id, state, "Ranged strike?", *self.get_id()).await?;
                if !strike {
                    return Ok(vec![]);
                }

                let picked_zone = LocationQuery::from_locations(path)
                    .with_prompt("Pick a zone to perform a ranged strike from")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state)
                    .await?;

                let direction = pick_direction(
                    controller_id,
                    &CARDINAL_DIRECTIONS,
                    state,
                    "Pick a direction for ranged strike",
                    *self.get_id(),
                )
                .await?;

                Ok(vec![Effect::ShootProjectile {
                    id: uuid::Uuid::new_v4(),
                    range: Some(self.ranged_range(state)?.unwrap_or(1)),
                    player_id: controller_id,
                    shooter: *self.get_id(),
                    origin: picked_zone,
                    direction,
                    damage: self
                        .get_power(state)?
                        .ok_or(anyhow::anyhow!("ranged attacker has no power"))?,
                    ranged_strike: true,
                    piercing: false,
                    splash_damage: None,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SkirmishersOfMu::NAME, |owner_id: PlayerId| {
        Box::new(SkirmishersOfMu::new(owner_id))
    });
