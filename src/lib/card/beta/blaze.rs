use crate::prelude::*;
use std::sync::Arc;

const ON_MOVE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct Blaze {
    card_base: CardBase,
    target_id: Option<CardId>,
}

impl Blaze {
    pub const NAME: &'static str = "Blaze";
    pub const DESCRIPTION: &'static str = "This turn, give an ally Movement +2, it can't be intercepted, and it leaves a trail of fire at departed locations. When it stops, each unit along the trail takes 2 damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            target_id: None,
        }
    }
}

#[async_trait::async_trait]
impl Card for Blaze {
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(data) = data.downcast_ref::<CardId>() {
            self.target_id = Some(*data);
        }

        Ok(())
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            ON_MOVE_HOOK => {
                let Some(target_id) = self.target_id else {
                    return Ok(vec![]);
                };

                let Effect::MoveCard { through_path, .. } = effect else {
                    return Ok(vec![]);
                };

                let mut effects = vec![];
                if let Some(path) = through_path {
                    for loc in path {
                        if Some(loc) != path.last() {
                            let units =
                                CardQuery::new().units().in_location(loc.clone()).all(state);
                            for unit_id in units {
                                effects.push(Effect::TakeDamage {
                                    card_id: unit_id,
                                    from: target_id,
                                    damage: Damage::basic(2),
                                });
                            }
                        }
                    }
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl Magic for Blaze {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let units = CardQuery::new()
            .units()
            .controlled_by(&self.get_controller_id(state))
            .all(state);
        let prompt = "Pick an ally";
        let picked_card = pick_card_source(
            self.get_controller_id(state),
            &units,
            state,
            prompt,
            Some(*self.get_id()),
        )
        .await?;
        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: picked_card,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Movement(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                },
            },
            Effect::SetCardData {
                card_id: *self.get_id(),
                data: Arc::new(picked_card),
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    hook_id: ON_MOVE_HOOK,
                    card_id: *self.get_id(),
                    timing: HookTiming::After,
                    trigger_on_effect: EffectQuery::MoveCard {
                        card: picked_card.into(),
                    },
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                    trigger_times: None,
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Blaze::NAME, |owner_id: PlayerId| {
    Box::new(Blaze::new(owner_id))
});
