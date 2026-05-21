use crate::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Constrict;

#[async_trait::async_trait]
impl ActivatedAbility for Constrict {
    fn get_name(&self) -> String {
        "Constrict".to_string()
    }

    fn get_cost(&self, card_id: &uuid::Uuid, _state: &State) -> anyhow::Result<Cost> {
        Ok(Cost::additional_only(AdditionalCost::tap(card_id)))
    }

    async fn on_select(
        &self,
        card_id: &uuid::Uuid,
        player_id: &PlayerId,
        state: &State,
    ) -> anyhow::Result<Vec<Effect>> {
        let constrictor = state.get_card(card_id);
        let mut effects = vec![];
        let mut constrict_zone = constrictor.get_zone().clone();

        if yes_or_no(player_id, state, "Tringh Constrictor: Take a step first?").await? {
            let mut zones = constrictor.get_zones_within_steps_of(state, 1, constrictor.get_zone());
            zones.retain(|zone| zone != constrictor.get_zone());
            if !zones.is_empty() {
                let picked_zone = pick_zone(
                    player_id,
                    &zones,
                    state,
                    false,
                    "Tringh Constrictor: Pick a location to step to",
                )
                .await?;
                effects.push(Effect::MoveCard {
                    player_id: *player_id,
                    card_id: *card_id,
                    from: constrictor.get_zone().clone(),
                    to: ZoneQuery::from_zone(picked_zone.clone()),
                    tap: true,
                    region: constrictor.get_region(state).clone(),
                    through_path: None,
                });
                constrict_zone = picked_zone;
            }
        }

        let Some(target_id) = CardQuery::new()
            .minions()
            .in_zone(&constrict_zone)
            .not_carried()
            .id_not(card_id)
            .with_prompt("Pick a minion to constrict")
            .with_source_card(*card_id)
            .pick(player_id, state, false)
            .await?
        else {
            return Ok(effects);
        };

        effects.push(Effect::SetBearer {
            card_id: target_id,
            bearer_id: Some(*card_id),
        });
        effects.push(Effect::AddAbilityCounter {
            card_id: target_id,
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Disabled,
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(target_id),
                }),
            },
        });

        let constrictor_id = *card_id;
        effects.push(Effect::AddTemporaryEffect {
            effect: TemporaryEffect::ModifyEffect {
                trigger_on_effect: EffectQuery::UntapCard {
                    card: CardQuery::from_id(constrictor_id),
                },
                expires_on_effect: EffectQuery::OneOf(vec![
                    EffectQuery::UntapCard {
                        card: CardQuery::from_id(constrictor_id),
                    },
                    EffectQuery::BuryCard {
                        card: CardQuery::from_id(target_id),
                    },
                    EffectQuery::BuryCard {
                        card: CardQuery::from_id(constrictor_id),
                    },
                ]),
                on_effect: Arc::new(move |state: &State, effect: &mut Effect| {
                    Box::pin(async move {
                        if state
                            .cards
                            .get(&target_id)
                            .and_then(|card| card.get_bearer_id().ok().flatten())
                            == Some(constrictor_id)
                        {
                            *effect = Effect::KillMinion {
                                card_id: target_id,
                                killer_id: constrictor_id,
                                from_attack: false,
                            };
                        }

                        Ok(())
                    })
                }),
            },
        });

        Ok(effects)
    }
}

#[derive(Debug, Clone)]
pub struct TringhConstrictor {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TringhConstrictor {
    pub const NAME: &'static str = "Tringh Constrictor";
    pub const DESCRIPTION: &'static str = "Tap → Tringh Constrictor may take a step, then it constricts target minion here and carries it disabled. The next time Tringh Constrictor would untap, it instead kills that minion if it's still constricted.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::CarryMinions(1)],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "W"),
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
impl Card for TringhConstrictor {
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
        Ok(vec![Box::new(Constrict)])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TringhConstrictor::NAME, |owner_id: PlayerId| {
        Box::new(TringhConstrictor::new(owner_id))
    });
