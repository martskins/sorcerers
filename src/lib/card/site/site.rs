use crate::{
    card::{site::Site, Card, CardBase, CardZone, Target},
    effect::{Action, Effect, GameAction},
    game::{Phase, State},
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteBase {
    pub card_base: CardBase,
    pub provided_mana: u8,
    pub provided_threshold: Thresholds,
}

impl Site {
    pub fn on_select(&self, _state: &State) -> Vec<Effect> {
        vec![]
    }

    pub fn get_cell_id(&self) -> Option<u8> {
        match self.get_zone() {
            CardZone::Realm(cell_id) => Some(*cell_id),
            _ => None,
        }
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

    pub fn deathrite(&self) -> Vec<Effect> {
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

        let cell_ids = state.valid_play_cells(&Card::Site(self.clone()));
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

    pub fn on_cast(&self, state: &State, target: Target) -> Vec<Effect> {
        let mut effects = Vec::new();
        match target {
            Target::Cell(cell_id) => effects.push(Effect::MoveCardToCell {
                card_id: self.get_id().clone(),
                cell_id,
            }),
            _ => {}
        }

        effects.extend(self.genesis());
        effects
    }
}
