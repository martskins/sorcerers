use crate::prelude::*;

const KILL_ALLIES_ON_DEATH: HookId = 1;

#[derive(Debug, Clone)]
pub struct AccursedAlbatross {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl AccursedAlbatross {
    pub const NAME: &'static str = "Accursed Albatross";
    pub const DESCRIPTION: &'static str = "Airborne

When a unit kills Accursed Albatross, kill that unit's other allied minions it's nearby.";

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
                costs: Costs::basic(3, "W"),
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
impl Card for AccursedAlbatross {
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
            id: KILL_ALLIES_ON_DEATH,
            trigger: EffectQuery::UnitKilled {
                unit: self.get_id().into(),
                killer: None,
            },
            timing: HookTiming::After,
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            KILL_ALLIES_ON_DEATH => {
                let Effect::KillMinion { killer_id, .. } = effect else {
                    return Ok(vec![]);
                };

                let mut effects = vec![];
                let killer = state.get_card(killer_id);
                let allies = CardQuery::new()
                    .minions()
                    .controlled_by(&killer.get_controller_id(state))
                    .near_to(killer.get_zone())
                    .all(state);
                for ally in allies {
                    if &ally == self.get_id() {
                        continue;
                    }

                    effects.push(Effect::KillMinion {
                        card_id: ally,
                        killer_id: *self.get_id(),
                        from_attack: false,
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
    (AccursedAlbatross::NAME, |owner_id: PlayerId| {
        Box::new(AccursedAlbatross::new(owner_id))
    });
