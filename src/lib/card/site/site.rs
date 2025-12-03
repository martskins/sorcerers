use crate::{
    card::{site::Site, Card, CardBase, CardZone, Target, Thresholds},
    effect::{Action, Effect, GameAction},
    game::{Phase, State},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteType {
    Desert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteBase {
    pub card_base: CardBase,
    pub provided_mana: u8,
    pub provided_threshold: Thresholds,
    pub site_types: Vec<SiteType>,
}

impl Site {
    pub fn on_select(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn get_square(&self) -> Option<u8> {
        match self.get_zone() {
            CardZone::Realm(square) => Some(*square),
            _ => None,
        }
    }

    pub fn genesis(&self, _state: &State) -> Vec<Effect> {
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

    pub fn deathrite(&self, state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn take_damage(&self, from: &uuid::Uuid, amount: u8) -> Vec<Effect> {
        vec![Effect::DealDamage {
            target_id: *self.get_id(),
            from: from.clone(),
            amount,
        }]
    }

    pub fn on_damage_taken(&self, from: &uuid::Uuid, amount: u8, _state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn on_prepare(&self, state: &State) -> Vec<Effect> {
        if !matches!(state.phase, Phase::SelectingCard { .. }) {
            return vec![];
        }

        let squares = state.valid_play_cells(&Card::Site(self.clone()));
        vec![Effect::ChangePhase {
            new_phase: Phase::SelectingSquare {
                player_id: self.get_owner_id().clone(),
                square: squares.clone(),
                after_select: Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets {
                    card_id: self.get_id().clone(),
                })),
            },
        }]
    }

    pub fn on_cast(&self, state: &State, target: Target) -> Vec<Effect> {
        let mut effects = Vec::new();
        match target {
            Target::Square(square) => effects.push(Effect::MoveCardToSquare {
                card_id: self.get_id().clone(),
                square,
            }),
            _ => {}
        }

        effects.extend(self.genesis(state));
        effects
    }
}
