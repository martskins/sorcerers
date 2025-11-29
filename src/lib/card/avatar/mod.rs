mod util;

use crate::{
    avatars,
    card::{CardBase, CardZone, Edition},
    effect::{Action, Effect, PlayerAction},
    game::{Phase, State},
};
use serde::{Deserialize, Serialize};

#[rustfmt::skip]
avatars! {
    Sorcerer, "Sorcerer", Edition::Beta,
    Battlemage, "Battlemage", Edition::Beta
}

impl Avatar {
    pub fn on_turn_start(&self, _: &State) -> Vec<Effect> {
        vec![Effect::UntapCard {
            card_id: self.get_id().clone(),
        }]
    }

    pub fn on_select(&self, _: &State) -> Vec<Effect> {
        if self.get_base().tapped {
            return vec![];
        }

        let actions = vec![
            Action::PlayerAction(PlayerAction::DrawSite {
                after_select: vec![Effect::TapCard {
                    card_id: self.get_id().clone(),
                }],
            }),
            Action::PlayerAction(PlayerAction::PlaySite {
                after_select: vec![Effect::TapCard {
                    card_id: self.get_id().clone(),
                }],
            }),
        ];

        vec![
            Effect::ChangePhase {
                new_phase: Phase::SelectingAction {
                    player_id: self.get_owner_id().clone(),
                    actions: actions.clone(),
                },
            },
            Effect::SetPlayerActions {
                player_id: self.get_owner_id().clone(),
                actions,
            },
        ]
    }
}
