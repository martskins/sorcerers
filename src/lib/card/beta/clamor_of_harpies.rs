use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";
    pub const DESCRIPTION: &'static str = "Airborne\r \r Genesis → Teleport target weaker minion to this location. Clamor of Harpies may strike it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Monster],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
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
impl Card for ClamorOfHarpies {
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
                let Some(card_id) = CardQuery::new()
                    .minions()
                    .power_lt(self.get_power(state)?.unwrap_or_default())
                    .with_source_card(*self.get_id())
                    .with_prompt("Pick a unit to bring here")
                    .pick(&self.get_controller_id(state), state)
                    .await?
                else {
                    return Ok(vec![]);
                };
                let card = state.get_card(&card_id);
                let strike = yes_or_no(
                    self.get_controller_id(state),
                    state,
                    "Strike selected unit?",
                    *self.get_id(),
                )
                .await?;
                let mut effects = vec![Effect::MoveCard {
                    player_id: self.get_controller_id(state),
                    card_id,
                    from: card.get_location().clone(),
                    to: self.get_location().clone().into(),
                    tap: false,
                    through_path: None,
                }];

                if strike {
                    effects.push(Effect::Strike {
                        striker_id: *self.get_id(),
                        target_id: card_id,
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
    (ClamorOfHarpies::NAME, |owner_id: PlayerId| {
        Box::new(ClamorOfHarpies::new(owner_id))
    });
