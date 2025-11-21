use serde::{Deserialize, Serialize};

use crate::{
    card::CardZone,
    game::{Resources, State},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddMana { player_id: uuid::Uuid, amount: u32 },
    CardMovedToCell { card_id: uuid::Uuid, cell_id: u8 },
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
            Effect::CardMovedToCell { card_id, cell_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.set_zone(CardZone::Realm(*cell_id));
                }
            }
        }
    }
}

pub enum Action {
    SelectCell { cell_ids: Vec<u8> },
}
