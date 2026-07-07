use crate::prelude::*;

const TAKE_DAMAGE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct DoomsdayProphet {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl DoomsdayProphet {
    pub const NAME: &'static str = "Doomsday Prophet";
    pub const DESCRIPTION: &'static str = "Nearby units take double damage, except from strikes.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "FF"),
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
impl Card for DoomsdayProphet {
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

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TAKE_DAMAGE_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: None,
                target: Some(Box::new(
                    CardQuery::new().units().nearby_to_card(self.get_id()),
                )),
            },
            timing: HookTiming::Replace,
            source_zones: HookSourceZones::InPlay,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            TAKE_DAMAGE_HOOK => {
                let Effect::TakeDamage {
                    card_id,
                    from,
                    damage,
                } = effect
                else {
                    return Ok(vec![]);
                };

                if damage.is_strike {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::TakeDamage {
                    card_id: *card_id,
                    from: *from,
                    damage: Damage {
                        amount: damage.amount * 2,
                        is_attack: damage.is_attack,
                        is_ranged: damage.is_ranged,
                        is_lethal: damage.is_lethal,
                        is_strike: damage.is_strike,
                    },
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (DoomsdayProphet::NAME, |owner_id: PlayerId| {
        Box::new(DoomsdayProphet::new(owner_id))
    });
