use serde::{Deserialize, Serialize};

use crate::{
    card::{Card, CardType, CardZone, Thresholds},
    game::{Phase, Resources, State},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddMana {
        player_id: uuid::Uuid,
        amount: u8,
    },
    AddThresholds {
        player_id: uuid::Uuid,
        thresholds: Thresholds,
    },
    MoveCardToSquare {
        card_id: uuid::Uuid,
        square: u8,
    },
    ChangePhase {
        new_phase: Phase,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
    UntapCard {
        card_id: uuid::Uuid,
    },
    DealDamage {
        target_id: uuid::Uuid,
        from: uuid::Uuid,
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
    DrawCard {
        player_id: uuid::Uuid,
        card_type: Option<CardType>,
    },
    KillUnit {
        card_id: uuid::Uuid,
    },
}

impl Effect {
    pub async fn apply(&self, state: &mut State) {
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
            Effect::MoveCardToSquare { card_id, square } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id);
                if let Some(card) = card {
                    card.set_zone(CardZone::Realm(*square));
                }
            }
            Effect::ChangePhase { new_phase } => {
                state.phase = new_phase.clone();
                if let Phase::SelectingAction { player_id, actions } = new_phase {
                    state.actions.insert(player_id.clone(), actions.clone());
                }
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
            Effect::DealDamage {
                from,
                target_id,
                amount,
            } => {
                let immutable_state = state.clone();
                let card = state.cards.iter_mut().find(|c| c.get_id() == target_id).unwrap();
                match card {
                    Card::Spell(spell) => {
                        spell.get_spell_base_mut().damage_taken += *amount;
                        let effects = spell.on_damage_taken(from, *amount, &immutable_state);
                        state.effects.extend(effects);
                    }
                    Card::Avatar(_) | Card::Site(_) => {
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
            Effect::DrawCard { player_id, card_type } => match card_type {
                Some(ct) => state.draw_card_for_player(player_id, ct.clone()).await.unwrap(),
                None => {
                    let new_phase = Phase::WaitingForCardDraw {
                        player_id: player_id.clone(),
                        count: 1,
                        types: vec![CardType::Spell, CardType::Site],
                    };
                    state.effects.push_back(Effect::ChangePhase { new_phase });
                }
            },
            Effect::KillUnit { card_id } => {
                let card = state.cards.iter().find(|c| c.get_id() == card_id).unwrap();
                let effects = card.deathrite(state);
                state.effects.extend(effects);
                state.effects.push_back(Effect::MoveCard {
                    card_id: *card_id,
                    to_zone: CardZone::Cemetery,
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerAction {
    DrawSite { after_select: Vec<Effect> },
    PlaySite { after_select: Vec<Effect> },
    Attack { after_select: Vec<Effect> },
    Move { after_select: Vec<Effect> },
    Defend { after_select: Vec<Effect> },
    ActivateTapAbility { after_select: Vec<Effect> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAction {
    SelectSquare { squares: Vec<u8> },
    SelectAction { actions: Vec<Action> },
    DrawCard { types: Vec<CardType> },
    PlayCardOnSelectedTargets { card_id: uuid::Uuid },
    PlaySelectedCard,
    AttackSelectedTarget { attacker_id: uuid::Uuid },
    MoveCardToSelectedSquare { card_id: uuid::Uuid },
    SummonMinionToSelectedSquare { card_id: uuid::Uuid },
    SummonMinion { card_id: uuid::Uuid },
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
            Action::PlayerAction(PlayerAction::Attack { .. }) => "Attack",
            Action::PlayerAction(PlayerAction::Move { .. }) => "Move",
            Action::PlayerAction(PlayerAction::Defend { .. }) => "Defend",
            Action::PlayerAction(PlayerAction::ActivateTapAbility { .. }) => "Activate Tap Ability",
            Action::GameAction(_) => "",
        }
    }

    pub fn after_select_effects(&self) -> Vec<Effect> {
        match self {
            Action::PlayerAction(PlayerAction::DrawSite { after_select }) => after_select.clone(),
            Action::PlayerAction(PlayerAction::PlaySite { after_select }) => after_select.clone(),
            Action::PlayerAction(PlayerAction::Attack { after_select }) => after_select.clone(),
            Action::PlayerAction(PlayerAction::Move { after_select }) => after_select.clone(),
            Action::PlayerAction(PlayerAction::Defend { after_select }) => after_select.clone(),
            Action::PlayerAction(PlayerAction::ActivateTapAbility { after_select }) => after_select.clone(),
            _ => vec![],
        }
    }
}
