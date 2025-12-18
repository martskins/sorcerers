use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Plane, UnitBase, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    ChoosingDirection,
    ChoosingUnit,
}

#[derive(Debug, Clone)]
pub struct ColickyDragonettes {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    status: Status,
}

impl ColickyDragonettes {
    pub const NAME: &'static str = "Colicky Dragonettes";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                modifiers: vec![],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("FF"),
                plane: Plane::Surface,
            },
            status: Status::None,
        }
    }
}

impl Card for ColickyDragonettes {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn on_turn_end(&self, state: &State) -> Vec<Effect> {
        let is_current_player = &state.current_player == self.get_owner_id();
        if !is_current_player {
            return vec![];
        }

        vec![
            Effect::set_card_status(self.get_id(), Status::ChoosingDirection),
            Effect::select_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS),
        ]
    }

    fn set_status(&mut self, status: &Box<dyn std::any::Any>) -> anyhow::Result<()> {
        let status = status
            .downcast_ref::<Status>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast status for {}", Self::NAME))?;
        self.status = status.clone();
        Ok(())
    }
}

impl MessageHandler for ColickyDragonettes {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        match (&self.status, message) {
            (Status::ChoosingDirection, ClientMessage::PickDirection { direction, .. }) => {
                let mut zone = self.get_zone().clone();
                loop {
                    zone = zone.zone_in_direction(&direction);
                    let units: Vec<uuid::Uuid> = state
                        .get_cards_in_zone(&zone)
                        .iter()
                        .filter(|c| c.is_unit())
                        .map(|c| c.get_id().clone())
                        .collect();
                    if units.is_empty() {
                        continue;
                    }

                    if units.len() == 1 {
                        return vec![
                            Effect::set_card_status(self.get_id(), Status::None),
                            Effect::take_damage(&units[0], self.get_id(), 1),
                            Effect::wait_for_play(self.get_owner_id()),
                        ];
                    }

                    return vec![
                        Effect::set_card_status(self.get_id(), Status::ChoosingUnit),
                        Effect::select_card(self.get_owner_id(), units, Some(self.get_id())),
                    ];
                }
            }
            (Status::ChoosingUnit, ClientMessage::PickCard { card_id, .. }) => {
                vec![
                    Effect::set_card_status(self.get_id(), Status::None),
                    Effect::take_damage(card_id, self.get_id(), 1),
                    Effect::wait_for_play(self.get_owner_id()),
                ]
            }
            _ => vec![],
        }
    }
}
