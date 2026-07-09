use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BaneWidow {
    card_base: CardBase,
    unit_base: UnitBase,
}

impl BaneWidow {
    pub const NAME: &'static str = "Bane Widow";
    pub const DESCRIPTION: &'static str = "Genesis -> May kill target minion here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                types: vec![MinionType::Beast],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
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
impl Card for BaneWidow {
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
        Ok(vec![Hook::genesis(self.get_id())])
    }

    async fn resolve_hook(
        &self,
        hook: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook {
            GENESIS_HOOK_ID => {
                let controller_id = self.get_controller_id(state);
                let target_query = CardQuery::new()
                    .minions()
                    .in_location(self.get_location().clone())
                    .id_not_in(vec![*self.get_id()]);
                if target_query.is_empty(state) {
                    return Ok(vec![]);
                }

                let use_genesis = yes_or_no(
                    &controller_id,
                    state,
                    "Kill a target minion here?",
                    *self.get_id(),
                )
                .await?;
                if !use_genesis {
                    return Ok(vec![]);
                };

                let Some(minion_id) = target_query
                    .with_prompt("Pick a minion to kill")
                    .with_source_card(*self.get_id())
                    .pick(&controller_id, state)
                    .await?
                else {
                    return Ok(vec![]);
                };

                Ok(vec![Effect::KillMinion {
                    card_id: minion_id,
                    killer_id: *self.get_id(),
                    from_attack: false,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (BaneWidow::NAME, |owner_id: PlayerId| {
    Box::new(BaneWidow::new(owner_id))
});
