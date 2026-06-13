use std::sync::Arc;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ShieldWall {
    card_base: CardBase,
}

impl ShieldWall {
    pub const NAME: &'static str = "Shield Wall";
    pub const DESCRIPTION: &'static str =
        "Until your next turn, each ally takes 1 less damage for each other ally it's nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for ShieldWall {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for ShieldWall {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        Ok(vec![Effect::AddTemporaryEffect {
            effect: TemporaryEffect::ModifyEffect {
                trigger_on_effect: EffectQuery::DamageDealt {
                    source: None,
                    target: Some(CardQuery::new().units().controlled_by(&controller_id)),
                },
                expires_on_effect: EffectQuery::TurnEnd {
                    player_id: Some(opponent_id),
                },
                on_effect: Arc::new(move |state: &State, effect: &mut Effect| {
                    Box::pin(async move {
                        if let &mut Effect::TakeDamage {
                            card_id,
                            ref mut damage,
                            ..
                        } = effect
                        {
                            let card = state.get_card(&card_id);
                            let allies_nearby = CardQuery::new()
                                .units()
                                .controlled_by(&controller_id)
                                .near_to(card.get_location())
                                .all(state)
                                .len();
                            damage.amount = damage.amount.saturating_sub(allies_nearby as u16);
                        }

                        Ok(())
                    })
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ShieldWall::NAME, |owner_id: PlayerId| {
    Box::new(ShieldWall::new(owner_id))
});
