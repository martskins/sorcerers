use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, Direction, PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    ChoosingDirection(uuid::Uuid),
    ChoosingFirstUnit(Zone, Direction),
    ChoosingSecondUnit,
}

#[derive(Debug, Clone)]
pub struct HeatRay {
    pub card_base: CardBase,
    status: Status,
}

impl HeatRay {
    pub const NAME: &'static str = "Heat Ray";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
            },
            status: Status::None,
        }
    }
}

impl Card for HeatRay {
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

    fn on_cast(&mut self, _state: &State, caster_id: &uuid::Uuid) -> Vec<Effect> {
        self.status = Status::ChoosingDirection(caster_id.clone());
        vec![Effect::select_direction(&self.get_owner_id(), &CARDINAL_DIRECTIONS)]
    }
}

impl MessageHandler for HeatRay {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        if message.player_id() != self.get_owner_id() {
            return vec![];
        }

        match (&self.status, message) {
            (Status::ChoosingDirection(caster_id), ClientMessage::PickDirection { direction, .. }) => {
                let caster = state.get_card(caster_id).unwrap();
                let square = caster.get_square().unwrap();
                let zone = match direction {
                    Direction::Up => Zone::Realm(square.saturating_add(5)),
                    Direction::Down => Zone::Realm(square.saturating_sub(5)),
                    Direction::Left => Zone::Realm(square.saturating_sub(1)),
                    Direction::Right => Zone::Realm(square.saturating_add(1)),
                };
                let valid_cards = state
                    .get_cards_in_zone(&zone)
                    .iter()
                    .filter(|c| c.is_unit())
                    .map(|c| c.get_id().clone())
                    .collect();
                self.status = Status::ChoosingFirstUnit(zone, direction.clone());
                vec![Effect::select_card(self.get_owner_id(), valid_cards)]
            }
            (Status::ChoosingFirstUnit(zone, direction), ClientMessage::PickCard { card_id, .. }) => {
                let square = zone.get_square().unwrap();
                let zone = match direction {
                    Direction::Up => Zone::Realm(square.saturating_add(5)),
                    Direction::Down => Zone::Realm(square.saturating_sub(5)),
                    Direction::Left => Zone::Realm(square.saturating_sub(1)),
                    Direction::Right => Zone::Realm(square.saturating_add(1)),
                };
                let valid_cards = state
                    .get_cards_in_zone(&zone)
                    .iter()
                    .filter(|c| c.is_unit())
                    .map(|c| c.get_id().clone())
                    .collect();
                self.status = Status::ChoosingSecondUnit;
                vec![
                    Effect::take_damage(card_id, self.get_id(), 2),
                    Effect::select_card(self.get_owner_id(), valid_cards),
                ]
            }
            (Status::ChoosingSecondUnit, ClientMessage::PickCard { card_id, .. }) => {
                self.status = Status::None;
                vec![
                    Effect::take_damage(card_id, self.get_id(), 2),
                    Effect::wait_for_play(self.get_owner_id()),
                ]
            }
            _ => vec![],
        }
    }
}
