use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Modifier, Plane, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    DrawingCard,
    PickingAlly,
}

#[derive(Debug, Clone)]
pub struct MadDash {
    pub card_base: CardBase,
    status: Status,
}

impl MadDash {
    pub const NAME: &'static str = "Mad Dash";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
            },
            status: Status::None,
        }
    }
}

impl Card for MadDash {
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

    fn on_cast(&mut self, _state: &State, _caster_id: &uuid::Uuid) -> Vec<Effect> {
        self.status = Status::DrawingCard;
        vec![Effect::wait_for_card_draw(&self.get_owner_id())]
    }
}

impl MessageHandler for MadDash {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        if message.player_id() != self.get_owner_id() {
            return vec![];
        }

        match (&self.status, message) {
            (Status::DrawingCard, ClientMessage::DrawCard { .. }) => {
                self.status = Status::PickingAlly;
                let valid_cards = state
                    .cards
                    .iter()
                    .filter(|c| c.get_owner_id() == self.get_owner_id())
                    .filter(|c| c.is_unit())
                    .filter(|c| matches!(c.get_zone(), Zone::Realm(_)))
                    .map(|c| c.get_id().clone())
                    .collect();
                vec![Effect::select_card(
                    self.get_owner_id(),
                    valid_cards,
                    Some(self.get_id()),
                )]
            }
            (Status::PickingAlly, ClientMessage::PickCard { card_id, .. }) => {
                self.status = Status::None;
                vec![
                    Effect::add_modifier(card_id, Modifier::Movement(1), Some(1)),
                    Effect::wait_for_play(self.get_owner_id()),
                ]
            }
            _ => vec![],
        }
    }
}
