use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{ActivatedAbility, PlayerId, UnitAction},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct TvinnaxBerserker {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TvinnaxBerserker {
    pub const NAME: &'static str = "Tvinnax Berserker";
    pub const DESCRIPTION: &'static str = "Whenever Tvinnax Berserker can attack a unit, he must. Untap Tvinnax Berserker whenever he attacks and kills an enemy minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
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
impl Card for TvinnaxBerserker {
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

    fn get_activated_abilities(
        &self,
        state: &State,
    ) -> anyhow::Result<Vec<Box<dyn ActivatedAbility>>> {
        if self.has_ability(state, &Ability::Disabled) {
            return Ok(vec![]);
        }

        if state.permanently_disabled_abilities.contains(self.get_id()) {
            return Ok(vec![]);
        }

        let mut abilities: Vec<Box<dyn ActivatedAbility>> =
            if !self.get_valid_attack_targets(state, false).is_empty() {
                vec![Box::new(UnitAction::Attack)]
            } else {
                self.base_unit_activated_abilities(state)?
            };
        abilities.extend(self.get_additional_activated_abilities(state)?);
        Ok(abilities)
    }

    fn on_attack(&self, state: &State, defender_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let defender = state.get_card(defender_id);
        if !defender.is_minion()
            || defender.get_controller_id(state) == self.get_controller_id(state)
        {
            return Ok(vec![]);
        }

        let self_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::BuryCard {
                    card: CardQuery::from_id(*defender_id),
                },
                expires_on_effect: Some(EffectQuery::OneOf(vec![
                    EffectQuery::TurnEnd {
                        player_id: Some(self.get_controller_id(state)),
                    },
                    EffectQuery::BuryCard {
                        card: CardQuery::from_id(self_id),
                    },
                ])),
                on_effect: Arc::new(
                    move |_state: &State, _card_id: &uuid::Uuid, _effect: &Effect| {
                        Box::pin(async move { Ok(vec![Effect::UntapCard { card_id: self_id }]) })
                            as Pin<
                                Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>,
                            >
                    },
                ),
                multitrigger: false,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TvinnaxBerserker::NAME, |owner_id: PlayerId| {
        Box::new(TvinnaxBerserker::new(owner_id))
    });
