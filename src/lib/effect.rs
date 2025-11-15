use serde::{Deserialize, Serialize};

use crate::game::{Resources, State};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddMana { player_id: uuid::Uuid, amount: u32 },
}

impl Effect {
    pub fn apply(&self, state: &mut State) {
        match self {
            Effect::AddMana { player_id, amount } => {
                let entry = state
                    .resources
                    .entry(*player_id)
                    .or_insert(Resources::new());
                entry.mana += *amount as u8;
            }
        }
    }
}
