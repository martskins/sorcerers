use crate::prelude::*;

const STRIKE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct CriticalStrike {
    card_base: CardBase,
}

impl CriticalStrike {
    pub const NAME: &'static str = "Critical Strike";
    pub const DESCRIPTION: &'static str =
        "The next time an ally strikes a unit this turn, it deals double damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: STRIKE_HOOK,
            trigger: EffectQuery::StrikeCard {
                card: CardQuery::new().units(),
                striker: Some(self.get_id().into()),
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::Any,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            STRIKE_HOOK => {
                let Effect::Strike {
                    striker_id,
                    target_id,
                } = effect
                else {
                    return Ok(vec![]);
                };

                // TODO: Double damage here once damage is included in Effect::Strike
                Ok(vec![Effect::Strike {
                    striker_id: *striker_id,
                    target_id: *target_id,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl Magic for CriticalStrike {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                hook_id: STRIKE_HOOK,
                card_id: *self.get_id(),
                trigger_on_effect: EffectQuery::StrikeCard {
                    card: CardQuery::new().units(),
                    striker: Some(CardQuery::new().minions().controlled_by(&controller_id)),
                },
                expires_on_effect: Some(EffectQuery::TurnEnd {
                    player_id: Some(controller_id),
                }),
                trigger_times: Some(1),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CriticalStrike::NAME, |owner_id: PlayerId| {
        Box::new(CriticalStrike::new(owner_id))
    });
