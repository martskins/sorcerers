use crate::{
    card::{CardType, Zone},
    game::PlayerStatus,
    state::State,
};
use std::fmt::Debug;

pub trait CardStatus: Debug + Send + Sync {}

#[derive(Debug, Clone)]
pub enum Effect {
    SetPlayerStatus {
        status: PlayerStatus,
    },
    PromptDecision {
        player_id: uuid::Uuid,
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
    PlayCard {
        player_id: uuid::Uuid,
        card_id: uuid::Uuid,
        square: u8,
    },
    TapCard {
        card_id: uuid::Uuid,
    },
}

impl Effect {
    pub fn apply(&self, state: &mut State) -> anyhow::Result<()> {
        match self {
            Effect::PromptDecision { player_id, options } => {
                state.player_status = PlayerStatus::SelectingAction {
                    player_id: player_id.clone(),
                    actions: options.clone(),
                };
            }
            Effect::MoveCard { card_id, to } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
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
                            .find(|c| c.get_id() == &card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Spell => {
                        let card_id = deck.spells.pop().unwrap();
                        state
                            .cards
                            .iter_mut()
                            .find(|c| c.get_id() == &card_id)
                            .unwrap()
                            .set_zone(Zone::Hand);
                    }
                    CardType::Avatar => unreachable!(),
                }
            }
            Effect::SetPlayerStatus { status, .. } => {
                state.player_status = status.clone();
            }
            Effect::PlayCard { card_id, square, .. } => {
                let snapshot = state.snapshot();
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.set_zone(Zone::Realm(*square));
                let effects = card.genesis(&snapshot);
                state.effects.extend(effects);
            }
            Effect::TapCard { card_id } => {
                let card = state.cards.iter_mut().find(|c| c.get_id() == card_id).unwrap();
                card.get_base_mut().tapped = true;
            }
        }

        Ok(())
    }
}
