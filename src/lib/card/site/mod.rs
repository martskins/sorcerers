mod util;

use crate::{
    card::{Card, CardBase, CardZone, Edition},
    effect::{Action, Effect, GameAction},
    game::{Phase, State},
    networking::Thresholds,
    sites,
};
use serde::{Deserialize, Serialize};

#[rustfmt::skip]
sites! {
    Aqueduct, "Aqueduct", 1, "", Edition::Beta,
    AridDesert, "Arid Desert", 1, "", Edition::Beta,
    AstralAlcazar, "Astral Alcazar", 2, "", Edition::Beta
}

impl Site {
    pub fn on_select(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn genesis(&self) -> Vec<Effect> {
        let mana = self.get_provided_mana();
        let thresholds = self.get_provided_threshold();
        vec![
            Effect::AddMana {
                player_id: self.get_owner_id().clone(),
                amount: mana,
            },
            Effect::AddThresholds {
                player_id: self.get_owner_id().clone(),
                thresholds,
            },
        ]
    }

    pub fn on_turn_start(&self, _: &State) -> Vec<Effect> {
        match self {
            _ => {
                vec![Effect::AddMana {
                    player_id: self.get_owner_id().clone(),
                    amount: 1,
                }]
            }
        }
    }

    pub fn on_prepare(&self, state: &State) -> Vec<Effect> {
        if !matches!(state.phase, Phase::SelectingCard { .. }) {
            return vec![];
        }

        let cell_ids = state.find_valid_cells_for_card(&Card::Site(self.clone()));
        vec![Effect::ChangePhase {
            new_phase: Phase::SelectingCell {
                player_id: self.get_owner_id().clone(),
                cell_ids: cell_ids.clone(),
                after_select: Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets {
                    card_id: self.get_id().clone(),
                })),
            },
        }]
    }
}
