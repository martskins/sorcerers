mod util;

use crate::{
    avatars,
    card::{CardBase, CardType, CardZone, Edition},
    effect::{Action, Effect, GameAction, PlayerAction},
    game::{Phase, State},
};
use serde::{Deserialize, Serialize};

#[rustfmt::skip]
avatars! {
    Sorcerer, "Sorcerer", Edition::Beta,
    Battlemage, "Battlemage", Edition::Beta,
    Flamecaller, "Flamecaller", Edition::Beta
}

impl Avatar {
    pub fn on_turn_start(&self, _: &State) -> Vec<Effect> {
        vec![Effect::UntapCard {
            card_id: self.get_id().clone(),
        }]
    }

    pub fn get_square(&self) -> Option<u8> {
        match self.get_zone() {
            CardZone::Realm(square) => Some(*square),
            _ => None,
        }
    }

    pub fn take_damage(&self, _from: &uuid::Uuid, _amount: u8) -> Vec<Effect> {
        vec![]
    }

    pub fn on_damage_taken(&self, _from: &uuid::Uuid, _amount: u8, _state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn on_select(&self, state: &State) -> Vec<Effect> {
        if self.get_base().tapped {
            return vec![];
        }

        let actions = vec![
            Action::PlayerAction(PlayerAction::DrawSite {
                after_select: vec![
                    Effect::TapCard {
                        card_id: self.get_id().clone(),
                    },
                    Effect::DrawCard {
                        player_id: self.get_owner_id().clone(),
                        card_type: Some(CardType::Site),
                    },
                    Effect::ChangePhase {
                        new_phase: Phase::WaitingForPlay {
                            player_id: self.get_owner_id().clone(),
                        },
                    },
                ],
            }),
            Action::PlayerAction(PlayerAction::PlaySite {
                after_select: vec![
                    Effect::ChangePhase {
                        new_phase: Phase::SelectingCard {
                            player_id: self.get_owner_id().clone(),
                            card_ids: state.get_playable_site_ids(self.get_owner_id()),
                            amount: 1,
                            after_select: Some(Action::GameAction(GameAction::PlaySelectedCard)),
                        },
                    },
                    Effect::TapCard {
                        card_id: self.get_id().clone(),
                    },
                ],
            }),
        ];

        vec![Effect::ChangePhase {
            new_phase: Phase::SelectingAction {
                player_id: self.get_owner_id().clone(),
                actions: actions.clone(),
            },
        }]
    }
}
