use serde::{Deserialize, Serialize};

use crate::{
    card::{CardType, CardZone},
    game::{Phase, Resources, State},
    networking::Thresholds,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddMana {
        player_id: uuid::Uuid,
        amount: u32,
    },
    AddThresholds {
        player_id: uuid::Uuid,
        thresholds: Thresholds,
    },
    CardMovedToCell {
        card_id: uuid::Uuid,
        cell_id: u8,
    },
    ChangePhase {
        new_phase: Phase,
    },
    SetPlayerActions {
        player_id: uuid::Uuid,
        actions: Vec<Action>,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
}

impl Effect {
    pub fn apply(&self, state: &mut State) {
        match self {
            Effect::AddMana { player_id, amount } => {
                let entry = state.resources.entry(*player_id).or_insert(Resources::new());
                entry.mana += *amount as u8;
            }
            Effect::AddThresholds { player_id, thresholds } => {
                let entry = state.resources.entry(*player_id).or_insert(Resources::new());
                entry.fire_threshold += thresholds.fire;
                entry.air_threshold += thresholds.air;
                entry.water_threshold += thresholds.water;
                entry.earth_threshold += thresholds.earth;
            }
            Effect::CardMovedToCell { card_id, cell_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.set_zone(CardZone::Realm(*cell_id));
                }
            }
            Effect::ChangePhase { new_phase } => {
                state.phase = new_phase.clone();
            }
            Effect::SetPlayerActions { player_id, actions } => {
                state.actions.insert(player_id.clone(), actions.clone());
            }
            Effect::TapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.tap();
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    SelectCell { cell_ids: Vec<u8> },
    SelectAction { actions: Vec<Action> },
    DrawCard { types: Vec<CardType> },
    DrawSite { after_select: Vec<Effect> },
    PlaySite { after_select: Vec<Effect> },
}

impl Action {
    pub fn get_name(&self) -> &'static str {
        match self {
            Action::SelectCell { .. } => "Select Cell",
            Action::SelectAction { .. } => "Select Action",
            Action::DrawCard { .. } => "Draw Card",
            Action::DrawSite { .. } => "Draw Site",
            Action::PlaySite { .. } => "Play Site",
        }
    }
}
