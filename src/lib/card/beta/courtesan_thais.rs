use crate::prelude::*;
use crate::state::Turn;

#[derive(Debug, Clone)]
pub struct CourtesanThais {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CourtesanThais {
    pub const NAME: &'static str = "Courtesan Thaïs";
    pub const DESCRIPTION: &'static str =
        "Genesis -> During their next turn, each player is controlled by the previous one.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 0,
                toughness: 0,
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
impl Card for CourtesanThais {
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
                let current_player = state.current_player();
                let next_player = state.next_turn().player_id();

                Ok(vec![
                    Effect::OverrideNextTurn {
                        turn: Turn::controlled_by(next_player, current_player),
                    },
                    Effect::OverrideNextTurn {
                        turn: Turn::controlled_by(current_player, next_player),
                    },
                ])
            }
            _ => Ok(vec![]),
        }
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CourtesanThais::NAME, |owner_id: PlayerId| {
        Box::new(CourtesanThais::new(owner_id))
    });
