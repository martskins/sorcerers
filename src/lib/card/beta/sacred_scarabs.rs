use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SacredScarabs {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SacredScarabs {
    pub const NAME: &'static str = "Sacred Scarabs";
    pub const DESCRIPTION: &'static str =
        "Airborne\r \r Deathrite → Deal 3 damage to each unit here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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
impl Card for SacredScarabs {
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

    async fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook::deathrite(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            DEATHRITE_HOOK_ID => {
                let Effect::TriggerDeathrite { from, .. } = effect else {
                    return Ok(vec![]);
                };
                let units_here: Vec<CardId> = state
                    .cards
                    .values()
                    .filter(|c| c.is_unit())
                    .filter(|c| c.get_zone() == from)
                    .map(|c| *c.get_id())
                    .collect();
                let mut effects = Vec::new();
                for unit in units_here {
                    effects.push(Effect::TakeDamage {
                        card_id: unit,
                        from: *self.get_id(),
                        damage: Damage::basic(3),
                    });
                }
                Ok(effects)
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SacredScarabs::NAME, |owner_id: PlayerId| {
        Box::new(SacredScarabs::new(owner_id))
    });
