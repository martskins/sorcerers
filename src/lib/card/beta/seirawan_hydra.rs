use crate::prelude::*;

const HEAL_NONLETHAL_DAMAGE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct SeirawanHydra {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SeirawanHydra {
    pub const NAME: &'static str = "Seirawan Hydra";
    pub const DESCRIPTION: &'static str = "Immediately heals from damage that doesn't kill it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 6,
                toughness: 6,
                types: vec![MinionType::Monster],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "W"),
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
impl Card for SeirawanHydra {
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
            id: HEAL_NONLETHAL_DAMAGE_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: None,
                target: Some(self.get_id().into()),
            },
            timing: HookTiming::After,
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
            // TODO: Not sure this behaves as it should. Write a test for it.
            HEAL_NONLETHAL_DAMAGE_HOOK => {
                let Effect::TakeDamage { damage, .. } = effect else {
                    return Ok(vec![]);
                };
                if damage.amount == 0 {
                    return Ok(vec![]);
                }

                let damage_taken = self.get_damage_taken()?;
                if damage_taken == 0 {
                    return Ok(vec![]);
                }

                Ok(vec![Effect::Heal {
                    card_id: *self.get_id(),
                    amount: damage_taken,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SeirawanHydra::NAME, |owner_id: PlayerId| {
        Box::new(SeirawanHydra::new(owner_id))
    });
