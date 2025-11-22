use crate::{
    card::{Card, CardBase, CardZone},
    effect::{Action, Effect},
    game::State,
    networking::Thresholds,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Site {
    Beacon(CardBase),
    Bog(CardBase),
    AnnualFair(CardBase),
}

impl Site {
    pub fn get_id(&self) -> &uuid::Uuid {
        match self {
            Site::Beacon(cb) => &cb.id,
            Site::Bog(cb) => &cb.id,
            Site::AnnualFair(cb) => &cb.id,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Site::Beacon(_) => "Beacon",
            Site::Bog(_) => "Bog",
            Site::AnnualFair(_) => "Annual Fair",
        }
    }

    pub fn get_owner_id(&self) -> &uuid::Uuid {
        match self {
            Site::Beacon(cb) => &cb.owner_id,
            Site::Bog(cb) => &cb.owner_id,
            Site::AnnualFair(cb) => &cb.owner_id,
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Site::Beacon(cb) => &cb.zone,
            Site::Bog(cb) => &cb.zone,
            Site::AnnualFair(cb) => &cb.zone,
        }
    }

    pub fn set_zone(&mut self, new_zone: CardZone) {
        match self {
            Site::Beacon(cb) => cb.zone = new_zone,
            Site::Bog(cb) => cb.zone = new_zone,
            Site::AnnualFair(cb) => cb.zone = new_zone,
        };
    }

    pub fn on_select(&self, state: &State) -> Vec<Action> {
        let cell_ids = state.find_valid_cells_for_card(&Card::Site(self.clone()));
        vec![Action::SelectCell { cell_ids }]
    }

    pub fn genesis(&self) -> Vec<Effect> {
        let mana = 1;
        let mut thresholds = Thresholds::zero();
        match self {
            Site::Beacon(_) => {
                thresholds.air = 1;
            }
            Site::Bog(_) => {
                thresholds.water = 1;
            }
            Site::AnnualFair(_) => {}
        }

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

    pub fn on_turn_start(&self) -> Vec<Effect> {
        match self {
            _ => {
                vec![Effect::AddMana {
                    player_id: self.get_owner_id().clone(),
                    amount: 1,
                }]
            }
        }
    }
}
