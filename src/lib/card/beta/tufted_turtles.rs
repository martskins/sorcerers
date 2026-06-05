use crate::prelude::*;

const PREVENT_FIRST_DAMAGE_HOOK: HookId = 1;
const TURN_START_HOOK: HookId = 2;

#[derive(Debug, Clone)]
pub struct TuftedTurtles {
    unit_base: UnitBase,
    card_base: CardBase,
    damage_prevented: bool,
}

impl TuftedTurtles {
    pub const NAME: &'static str = "Tufted Turtles";
    pub const DESCRIPTION: &'static str =
        "The first time Tufted Turtles would take damage each turn, prevent that damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "W"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            damage_prevented: false,
        }
    }
}

#[async_trait::async_trait]
impl Card for TuftedTurtles {
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

    fn set_data(
        &mut self,
        _data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(damage_prevented) = _data.downcast_ref::<bool>() {
            self.damage_prevented = *damage_prevented;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid data type for Tufted Turtles"))
        }
    }

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        if !self.get_zone().is_in_play() {
            return Ok(vec![]);
        }

        let mut hooks = vec![Hook {
            id: TURN_START_HOOK,
            trigger: EffectQuery::TurnStart { player_id: None },
            timing: HookTiming::After,
            source_zones: HookSourceZones::InPlay,
        }];

        if self.damage_prevented {
            return Ok(hooks);
        }

        hooks.push(Hook {
            id: PREVENT_FIRST_DAMAGE_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: None,
                target: Some(self.get_id().into()),
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::InPlay,
        });

        Ok(hooks)
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            PREVENT_FIRST_DAMAGE_HOOK => {
                if self.damage_prevented {
                    return Ok(vec![]);
                }

                if let Effect::TakeDamage {
                    card_id, damage, ..
                } = effect
                    && card_id == self.get_id()
                    && damage.amount > 0
                {
                    return Ok(vec![Effect::SetCardData {
                        card_id: *self.get_id(),
                        data: std::sync::Arc::new(true),
                    }]);
                }

                Ok(vec![])
            }
            TURN_START_HOOK => Ok(vec![Effect::SetCardData {
                card_id: *self.get_id(),
                data: std::sync::Arc::new(false),
            }]),
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (TuftedTurtles::NAME, |owner_id: PlayerId| {
        Box::new(TuftedTurtles::new(owner_id))
    });
