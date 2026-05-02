use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, AbilityCounter, Card, CardBase, CardConstructor, Costs, Edition, MinionType,
        Rarity, Region, UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_card, yes_or_no},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct UnlandEel {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl UnlandEel {
    pub const NAME: &'static str = "Unland Eel";
    pub const DESCRIPTION: &'static str =
        "Submerge Whenever Unland Eel submerges, it may drag another minion here down with it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Submerge],
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for UnlandEel {
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

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let self_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::SetCardRegion {
                    card: CardQuery::from_id(self_id),
                    region: Some(Region::Underwater),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, _card_id: &uuid::Uuid, _effect: &Effect| {
                        Box::pin(async move {
                            let eel = state.get_card(&self_id);
                            let controller_id = eel.get_controller_id(state);
                            let other_minions = CardQuery::new()
                                .minions()
                                .in_zone(eel.get_zone())
                                .id_not(&self_id)
                                .all(state);
                            if other_minions.is_empty() {
                                return Ok(vec![]);
                            }

                            if !yes_or_no(
                                &controller_id,
                                state,
                                "Unland Eel: Drag another minion down with it?",
                            )
                            .await?
                            {
                                return Ok(vec![]);
                            }

                            let target_id = pick_card(
                                &controller_id,
                                &other_minions,
                                state,
                                "Unland Eel: Pick another minion here to drag down",
                            )
                            .await?;
                            let target = state.get_card(&target_id);

                            let mut effects = vec![];
                            if target.get_region(state) != &Region::Underwater
                                && !target.has_ability(state, &Ability::Submerge)
                            {
                                effects.push(Effect::AddAbilityCounter {
                                    card_id: target_id,
                                    counter: AbilityCounter {
                                        id: uuid::Uuid::new_v4(),
                                        ability: Ability::Submerge,
                                        expires_on_effect: Some(EffectQuery::SetCardRegion {
                                            card: CardQuery::from_id(target_id),
                                            region: Some(Region::Surface),
                                        }),
                                    },
                                });
                            }
                            effects.push(Effect::SetCardRegion {
                                card_id: target_id,
                                region: Region::Underwater,
                                tap: false,
                            });
                            Ok(effects)
                        })
                            as Pin<
                                Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                            >
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (UnlandEel::NAME, |owner_id: PlayerId| {
    Box::new(UnlandEel::new(owner_id))
});
