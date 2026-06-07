use crate::prelude::*;

const DESTROY_ON_STRIKE: HookId = 1;

#[derive(Debug, Clone)]
pub struct PhantasmalShade {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl PhantasmalShade {
    pub const NAME: &'static str = "Phantasmal Shade";
    pub const DESCRIPTION: &'static str = "When Phantasmal Shade is struck, destroy it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Voidwalk, Ability::Stealth],
                types: vec![MinionType::Spirit],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "AA"),
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
impl Card for PhantasmalShade {
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
            id: DESTROY_ON_STRIKE,
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
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            // TODO: Should this be a BuryCard instead?
            DESTROY_ON_STRIKE => Ok(vec![Effect::KillMinion {
                card_id: *self.get_id(),
                killer_id: *self.get_id(),
                from_attack: false,
            }]),
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PhantasmalShade::NAME, |owner_id: PlayerId| {
        Box::new(PhantasmalShade::new(owner_id))
    });
