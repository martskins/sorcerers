use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Plane, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, InputStatus, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HeatRay {
    pub card_base: CardBase,
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
                plane: Plane::Surface,
            },
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

    fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> Vec<Effect> {
        let caster = state.get_card(caster_id).unwrap();
        let from = caster.get_zone();
        vec![
            Effect::set_input_status(InputStatus::ShootingProjectile {
                player_id: self.get_owner_id().clone(),
                card_id: self.get_id().clone(),
                caster_id: Some(caster_id.clone()),
                from: from.clone(),
                direction: None,
                damage: 2,
                piercing: true,
            }),
            Effect::select_direction(&self.get_owner_id(), &CARDINAL_DIRECTIONS),
        ]
    }
}

impl MessageHandler for HeatRay {
    // fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
    //     if message.player_id() != self.get_owner_id() {
    //         return vec![];
    //     }
    //
    //     match (&self.status, message) {
    //         (Status::ChoosingDirection(caster_id), ClientMessage::PickDirection { direction, .. }) => {
    //             let caster = state.get_card(caster_id).unwrap();
    //             let zone = caster.get_zone().zone_in_direction(direction);
    //             let valid_cards = state
    //                 .get_cards_in_zone(&zone)
    //                 .iter()
    //                 .filter(|c| c.is_unit())
    //                 .map(|c| c.get_id().clone())
    //                 .collect();
    //             self.status = Status::ChoosingFirstUnit(zone, direction.clone());
    //             vec![Effect::select_card(
    //                 self.get_owner_id(),
    //                 valid_cards,
    //                 Some(self.get_id()),
    //             )]
    //         }
    //         (Status::ChoosingFirstUnit(zone, direction), ClientMessage::PickCard { card_id, .. }) => {
    //             let zone = zone.zone_in_direction(direction);
    //             let valid_cards = state
    //                 .get_cards_in_zone(&zone)
    //                 .iter()
    //                 .filter(|c| c.is_unit())
    //                 .map(|c| c.get_id().clone())
    //                 .collect();
    //             self.status = Status::ChoosingSecondUnit;
    //             vec![
    //                 Effect::take_damage(card_id, self.get_id(), 2),
    //                 Effect::select_card(self.get_owner_id(), valid_cards, Some(self.get_id())),
    //             ]
    //         }
    //         (Status::ChoosingSecondUnit, ClientMessage::PickCard { card_id, .. }) => {
    //             self.status = Status::None;
    //             vec![
    //                 Effect::take_damage(card_id, self.get_id(), 2),
    //                 Effect::wait_for_play(self.get_owner_id()),
    //             ]
    //         }
    //         _ => vec![],
    //     }
    // }
}
