use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct MordricDruids {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MordricDruids {
    pub const NAME: &'static str = "Mordric Druids";
    pub const DESCRIPTION: &'static str = "Spellcaster\r \r Whenever a nearby enemy attacks your Avatar, that attacker takes the same damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Spellcaster(None)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for MordricDruids {
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

    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let druids_id = *self.get_id();
        let controller_id = self.get_controller_id(state);

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::DamageDealt {
                    source: None,
                    target: Some(CardQuery::new().avatars().controlled_by(&controller_id)),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(druids_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, _avatar_id: &uuid::Uuid, effect: &Effect| {
                        Box::pin(async move {
                            let Effect::TakeDamage {
                                from: attacker_id,
                                damage,
                                ..
                            } = effect
                            else {
                                return Ok(vec![]);
                            };

                            // Only trigger if the attacker is nearby the Mordric Druids
                            let druids = state.get_card(&druids_id);
                            if !druids.get_zone().is_in_play() {
                                return Ok(vec![]);
                            }
                            let druids_zone = druids.get_zone().clone();
                            let attacker = state.get_card(attacker_id);
                            let attacker_zone = attacker.get_zone().clone();

                            let is_nearby = druids_zone == attacker_zone
                                || druids_zone.get_adjacent().contains(&attacker_zone);
                            if !is_nearby {
                                return Ok(vec![]);
                            }

                            // Ensure it's actually an enemy
                            let attacker_controller = attacker.get_controller_id(state);
                            if attacker_controller == controller_id {
                                return Ok(vec![]);
                            }

                            Ok(vec![Effect::TakeDamage {
                                card_id: *attacker_id,
                                from: druids_id,
                                damage: *damage,
                                is_strike: false,
                                is_ranged: false,
                            }])
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MordricDruids::NAME, |owner_id: PlayerId| {
        Box::new(MordricDruids::new(owner_id))
    });
