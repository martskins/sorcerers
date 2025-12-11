use crate::{
    card::{AvatarStatus, CardType, Zone},
    game::PlayerStatus,
    state::State,
};
use std::fmt::Debug;

pub trait CardStatus: Debug + Send + Sync {}

#[derive(Debug)]
pub enum Effect {
    SetPlayerStatus {
        status: PlayerStatus,
    },
    SetAvatarStatus {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        status: AvatarStatus,
    },
    PromptDecision {
        player_id: uuid::Uuid,
        source_id: Option<uuid::Uuid>,
        options: Vec<String>,
    },
    MoveCard {
        card_id: uuid::Uuid,
        to: Zone,
    },
    DrawCard {
        player_id: uuid::Uuid,
        card_type: CardType,
    },
    SetCardStatus {
        card_id: uuid::Uuid,
        status: Box<dyn CardStatus>,
    },
}

impl Effect {
    pub fn apply(&self, state: &mut State) -> anyhow::Result<()> {
        match self {
            Effect::PromptDecision {
                player_id,
                options,
                source_id,
            } => {
                state.player_status = PlayerStatus::SelectingAction {
                    player_id: player_id.clone(),
                    source_id: source_id.clone(),
                    actions: options.clone(),
                };
            }
            Effect::MoveCard { card_id, to } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == *card_id).unwrap();
                card.set_zone(to.clone());
            }
            Effect::DrawCard { player_id, card_type } => {
                let deck = state.decks.get_mut(player_id).unwrap();
                match card_type {
                    CardType::Site => {
                        let card_id = deck.sites.pop().unwrap();
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Spell => {
                        let card_id = deck.spells.pop().unwrap();
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Avatar => unreachable!(),
                }
            }
            Effect::SetCardStatus { card_id, status } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == *card_id).unwrap();
            }
            Effect::SetAvatarStatus { card_id, status, .. } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == *card_id).unwrap();
                card.set_status(status.clone());
            }
            Effect::SetPlayerStatus { status, .. } => {
                state.player_status = status.clone();
            }
        }

        Ok(())
    }
}
