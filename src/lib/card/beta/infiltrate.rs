use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct Infiltrate {
    card_base: CardBase,
}

impl Infiltrate {
    pub const NAME: &'static str = "Infiltrate";
    pub const DESCRIPTION: &'static str = "Target enemy minion gains Stealth and taps. You control it until it no longer has Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
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
impl Card for Infiltrate {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_zone = caster.get_zone().clone();

        let enemy_minions: Vec<uuid::Uuid> = CardQuery::new()
            .minions()
            .near_to(&caster_zone)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect();

        if enemy_minions.is_empty() {
            return Ok(vec![]);
        }

        let Some(target_id) = CardQuery::from_ids(enemy_minions)
            .with_prompt("Infiltrate: Pick target enemy minion")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        let original_controller = state.get_card(&target_id).get_controller_id(state);
        let new_controller = controller_id;
        let stealth_counter_id = uuid::Uuid::new_v4();

        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: target_id,
                counter: AbilityCounter {
                    id: stealth_counter_id,
                    ability: Ability::Stealth,
                    expires_on_effect: None,
                },
            },
            Effect::TapCard { card_id: target_id },
            Effect::SetController {
                card_id: target_id,
                player_id: new_controller,
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::TurnEnd { player_id: None },
                    expires_on_effect: Some(EffectQuery::BuryCard {
                        card: target_id.into(),
                    }),
                    multitrigger: true,
                    on_effect: Arc::new(
                        move |state: &State, _triggered_card_id: &uuid::Uuid, _effect: &Effect| {
                            Box::pin(async move {
                                let target = state.get_card(&target_id);
                                if !target.has_ability(state, &Ability::Stealth) {
                                    return Ok(vec![Effect::SetController {
                                        card_id: target_id,
                                        player_id: original_controller,
                                    }]);
                                }
                                Ok(vec![])
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
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Infiltrate::NAME, |owner_id: PlayerId| {
    Box::new(Infiltrate::new(owner_id))
});
