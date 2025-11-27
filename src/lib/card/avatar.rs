use crate::{
    card::{CardBase, CardZone},
    effect::{Action, Effect, PlayerAction},
    game::{Phase, State},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Avatar {
    Sorcerer(CardBase),
    Battlemage(CardBase),
}

impl Avatar {
    pub fn get_base(&self) -> &CardBase {
        match self {
            Avatar::Sorcerer(cb) => cb,
            Avatar::Battlemage(cb) => cb,
        }
    }

    pub fn get_base_mut(&mut self) -> &mut CardBase {
        match self {
            Avatar::Sorcerer(cb) => cb,
            Avatar::Battlemage(cb) => cb,
        }
    }

    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Avatar::Sorcerer(cb) => &cb.id,
            Avatar::Battlemage(cb) => &cb.id,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Avatar::Sorcerer(_) => "Sorcerer",
            Avatar::Battlemage(_) => "Battlemage",
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Avatar::Sorcerer(cb) => &cb.owner_id,
            Avatar::Battlemage(cb) => &cb.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Avatar::Sorcerer(cb) => &cb.zone,
            Avatar::Battlemage(cb) => &cb.zone,
        }
    }

    pub fn set_zone(&mut self, zone: CardZone) {
        match self {
            Avatar::Sorcerer(cb) => cb.zone = zone,
            Avatar::Battlemage(cb) => cb.zone = zone,
        };
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
