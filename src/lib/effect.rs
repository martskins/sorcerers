use serde::{Deserialize, Serialize};

use crate::game::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddMana { player_id: uuid::Uuid, amount: u32 },
}

impl Effect {
    pub fn apply(&self, state: &mut State) {
        match self {
            Effect::AddMana { player_id, amount } => {
                let entry = state.player_mana.entry(*player_id).or_insert(0);
                *entry += *amount as u8;
            }
        }
    }
}
