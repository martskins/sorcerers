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
    MoveCardToCell {
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
    UntapCard {
        card_id: uuid::Uuid,
    },
    DealDamage {
        target_id: uuid::Uuid,
        amount: u8,
    },
    SpendMana {
        player_id: uuid::Uuid,
        amount: u8,
    },
    MoveCard {
        card_id: uuid::Uuid,
        to_zone: CardZone,
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
            Effect::MoveCardToCell { card_id, cell_id } => {
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
            Effect::UntapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.untap();
                }
            }
            Effect::TapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.tap();
                }
            }
            Effect::DealDamage { target_id, amount } => {
                let card = state.cards.iter().find(|c| c.get_id() == target_id).unwrap();
                match card.get_type() {
                    CardType::Spell => {
                        // TODO: implement damage taking
                        state.effects.push_back(Effect::MoveCard {
                            card_id: *target_id,
                            to_zone: CardZone::DiscardPile,
                        });
                    }
                    CardType::Avatar | CardType::Site => {
                        println!("Dealing {} damage to player {:?}", amount, card.get_owner_id());
                        state.resources.get_mut(card.get_owner_id()).unwrap().health -= amount;
                    }
                }
            }
            Effect::SpendMana { player_id, amount } => {
                let entry = state.resources.entry(*player_id).or_insert(Resources::new());
                entry.mana = entry.mana.saturating_sub(*amount);
            }
            Effect::MoveCard { card_id, to_zone } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.set_zone(to_zone.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerAction {
    DrawSite { after_select: Vec<Effect> },
    PlaySite { after_select: Vec<Effect> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    SelectCell { cell_ids: Vec<u8> },
    SelectAction { actions: Vec<Action> },
    DrawCard { types: Vec<CardType> },
    PlayCardOnSelectedTargets { card_id: uuid::Uuid },
    PlaySelectedCard,
    AttackSelectedTarget { attacker_id: uuid::Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    GameAction(GameAction),
    PlayerAction(PlayerAction),
}

impl Action {
    pub fn get_name(&self) -> &'static str {
        match self {
            Action::PlayerAction(PlayerAction::DrawSite { .. }) => "Draw Site",
            Action::PlayerAction(PlayerAction::PlaySite { .. }) => "Play Site",
            Action::GameAction(_) => "",
        }
    }
}
