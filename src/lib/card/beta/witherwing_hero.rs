use std::{future::Future, pin::Pin, sync::Arc};

use crate::prelude::*;

const ON_SUMMON_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct WitherwingHero {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl WitherwingHero {
    pub const NAME: &'static str = "Witherwing Hero";
    pub const DESCRIPTION: &'static str = "Airborne
        Whenever a weaker allied minion here is attacked, you may return it to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for WitherwingHero {
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
            id: ON_SUMMON_HOOK,
            trigger: EffectQuery::SummonCard {
                card: self.get_id().into(),
            },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ON_SUMMON_HOOK => {
                let self_id = *self.get_id();
                Ok(vec![Effect::AddDeferredEffect {
                    effect: DeferredEffect {
                        trigger_on_effect: EffectQuery::Attack {
                            attacker: CardQuery::new(),
                            defender: None,
                        },
                        expires_on_effect: Some(EffectQuery::BuryCard {
                            card: CardQuery::from_id(self_id),
                        }),
                        on_effect: Arc::new(
                            move |state: &State, _card_id: &CardId, effect: &Effect| {
                                Box::pin(async move {
                                    let (attacker_id, defender_id) = match effect {
                                        Effect::Attack {
                                            attacker_id,
                                            defender_id,
                                            ..
                                        } => (*attacker_id, *defender_id),
                                        _ => return Ok(vec![]),
                                    };
                                    let self_card = state.get_card(&self_id);
                                    if !self_card.get_zone().is_in_play() {
                                        return Ok(vec![]);
                                    }
                                    let hero_controller = self_card.get_controller_id(state);
                                    let hero_zone = self_card.get_zone().clone();
                                    let hero_power =
                                        self_card.get_unit_base().map(|ub| ub.power).unwrap_or(0);
                                    let defender = state.get_card(&defender_id);
                                    let defender_controller = defender.get_controller_id(state);
                                    if defender_controller != hero_controller {
                                        return Ok(vec![]);
                                    }
                                    if *defender.get_zone() != hero_zone {
                                        return Ok(vec![]);
                                    }
                                    let defender_power =
                                        defender.get_unit_base().map(|ub| ub.power).unwrap_or(0);
                                    if defender_power >= hero_power {
                                        return Ok(vec![]);
                                    }
                                    let _ = attacker_id;
                                    let should_return = yes_or_no_source(
                                        &hero_controller,
                                        state,
                                        "Return the attacked ally to its owner's hand?",
                                        Some(self_id),
                                    )
                                    .await?;
                                    if !should_return {
                                        return Ok(vec![]);
                                    }
                                    Ok(vec![Effect::SetCardZone {
                                        card_id: defender_id,
                                        zone: Zone::Hand,
                                    }])
                                })
                                    as Pin<
                                        Box<
                                            dyn Future<Output = anyhow::Result<Vec<Effect>>>
                                                + Send
                                                + '_,
                                        >,
                                    >
                            },
                        ),
                        multitrigger: true,
                    },
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WitherwingHero::NAME, |owner_id: PlayerId| {
        Box::new(WitherwingHero::new(owner_id))
    });
