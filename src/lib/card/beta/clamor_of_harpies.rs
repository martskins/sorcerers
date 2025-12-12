use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, PlayerStatus},
    networking::message::ClientMessage,
    state::State,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Status {
    None,
    MinionPick,
    StrikeDecision,
}

#[derive(Debug, Clone)]
pub struct ClamorOfHarpies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    pub targeted_minion: uuid::Uuid,
    status: Status,
}

impl ClamorOfHarpies {
    pub const NAME: &'static str = "Clamor of Harpies";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase { power: 3, toughness: 3 },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
            },
            status: Status::None,
            targeted_minion: uuid::Uuid::nil(),
        }
    }
}

impl Card for ClamorOfHarpies {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn is_tapped(&self) -> bool {
        self.card_base.tapped
    }

    fn get_owner_id(&self) -> &PlayerId {
        &self.card_base.owner_id
    }

    fn get_edition(&self) -> Edition {
        Edition::Beta
    }

    fn get_id(&self) -> &uuid::Uuid {
        &self.card_base.id
    }

    fn get_card_type(&self) -> crate::card::CardType {
        CardType::Spell
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn genesis(&mut self, state: &State) -> Vec<Effect> {
        self.status = Status::MinionPick;
        let valid_cards = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| match c.get_unit_base() {
                Some(ub) => ub.toughness < self.unit_base.power,
                _ => false,
            })
            .map(|c| c.get_id().clone())
            .collect();
        vec![Effect::SetPlayerStatus {
            status: PlayerStatus::SelectingCard {
                player_id: self.get_owner_id().clone(),
                valid_cards,
            },
        }]
    }
}

impl MessageHandler for ClamorOfHarpies {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match (&self.status, message) {
            (Status::MinionPick, ClientMessage::PickCard { card_id, .. }) => {
                self.targeted_minion = card_id.clone();
                self.status = Status::StrikeDecision;
                vec![]
            }
            (Status::StrikeDecision, ClientMessage::PickAction { action_idx, .. }) => {
                self.status = Status::None;
                vec![]
            }
            _ => vec![],
        }
    }
}
