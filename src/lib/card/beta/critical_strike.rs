use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct CriticalStrike {
    pub card_base: CardBase,
}

impl CriticalStrike {
    pub const NAME: &'static str = "Critical Strike";
    pub const DESCRIPTION: &'static str = "The next time an ally strikes a unit this turn, it deals double damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for CriticalStrike {
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
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::DamageDealt {
                    source: Some(CardQuery::new().minions().controlled_by(&controller_id)),
                    target: None,
                },
                expires_on_effect: Some(EffectQuery::TurnEnd {
                    player_id: Some(controller_id.clone()),
                }),
                on_effect: Arc::new(move |_state: &State, _card_id: &uuid::Uuid, effect: &Effect| {
                    Box::pin(async move {
                        if let Effect::TakeDamage { card_id, from, damage } = effect {
                            Ok(vec![Effect::TakeDamage {
                                card_id: card_id.clone(),
                                from: from.clone(),
                                damage: *damage,
                            }])
                        } else {
                            Ok(vec![])
                        }
                    }) as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                }),
                multitrigger: false,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (CriticalStrike::NAME, |owner_id: PlayerId| {
    Box::new(CriticalStrike::new(owner_id))
});
