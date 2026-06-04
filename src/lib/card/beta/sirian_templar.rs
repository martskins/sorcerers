use crate::prelude::*;

const PREVENT_DAMAGE_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct SirianTemplar {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SirianTemplar {
    pub const NAME: &'static str = "Sirian Templar";
    pub const DESCRIPTION: &'static str = "Takes no damage from Demon, Spirit, or Undead minions.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "EE"),
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
impl Card for SirianTemplar {
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
        Ok(vec![Hook {
            id: PREVENT_DAMAGE_HOOK,
            trigger: EffectQuery::DamageDealt {
                source: Some(CardQuery::new().minions().minion_types(vec![
                    MinionType::Demon,
                    MinionType::Spirit,
                    MinionType::Undead,
                ])),
                target: Some(self.get_id().into()),
            },
            timing: HookTiming::Replace,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        _state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            PREVENT_DAMAGE_HOOK => Ok(vec![Effect::Noop]),
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (SirianTemplar::NAME, |owner_id: PlayerId| {
        Box::new(SirianTemplar::new(owner_id))
    });
