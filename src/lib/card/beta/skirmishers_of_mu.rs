use crate::prelude::*;
use std::sync::Arc;

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

    async fn hooks(&self, state: &State) -> anyhow::Result<Vec<Hook>> {
        let self_id = *self.get_id();
        let controller_id = self.get_controller_id(state);
        Ok(vec![Hook {
            trigger: EffectQuery::MoveCard {
                card: self.get_id().into(),
            },
            timing: HookTiming::After,
            action: HookAction::Callback(Arc::new(move |state: &State, effect: &Effect| {
                Box::pin(async move {
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

                    let skirmishers = state.get_card(&self_id);
                    let to = to.pick(player_id, state).await?.into_zone();
                    let mut path = vec![from.clone().into_zone(), to];
                    if let Some(through_path) = through_path {
                        path = through_path.to_vec();
                    }

                    let options = [BaseOption::Yes, BaseOption::No];
                    let option_labels = options.iter().map(|o| o.to_string()).collect::<Vec<_>>();
                    let picked_option = pick_option(
                        controller_id,
                        &option_labels,
                        state,
                        "Ranged strike?",
                        false,
                    )
                    .await?;
                    if options[picked_option] == BaseOption::No {
                        return Ok(vec![]);
                    }

                    let picked_zone = pick_zone(
                        controller_id,
                        &path,
                        state,
                        false,
                        "Skirmishers of Mu: Pick a zone to perform a ranged strike from",
                    )
                    .await?;

                    let direction = pick_direction_source(
                        controller_id,
                        &CARDINAL_DIRECTIONS,
                        state,
                        "Skirmishers of Mu: Pick a direction for ranged strike",
                        Some(self_id),
                    )
                    .await?;

                    let mut effects = skirmishers.after_ranged_attack(state).await?;
                    effects.push(Effect::ShootProjectile {
                        id: uuid::Uuid::new_v4(),
                        range: Some(skirmishers.ranged_range(state)?.unwrap_or(1)),
                        player_id: controller_id,
                        shooter: self_id,
                        from_zone: picked_zone,
                        direction,
                        damage: skirmishers
                            .get_power(state)?
                            .ok_or(anyhow::anyhow!("ranged attacker has no power"))?,
                        ranged_strike: true,
                        piercing: false,
                        splash_damage: None,
                    });
                    effects.push(Effect::RemoveAbility {
                        card_id: self_id,
                        modifier: Ability::Stealth,
                    });

                    Ok(effects)
                })
            })),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SkirmishersOfMu::NAME, |owner_id: PlayerId| {
        Box::new(SkirmishersOfMu::new(owner_id))
    });
