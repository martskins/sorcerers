use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Plane, Zone},
    effect::Effect,
    game::{Direction, PlayerId, Thresholds},
    networking::message::ClientMessage,
    state::State,
};

#[derive(Debug, Clone)]
enum Status {
    None,
    SelectingZone,
}

#[derive(Debug, Clone)]
pub struct MajorExplosion {
    pub card_base: CardBase,
    status: Status,
}

impl MajorExplosion {
    pub const NAME: &'static str = "Major Explosion";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 7,
                required_thresholds: Thresholds::parse("FF"),
                plane: Plane::Surface,
            },
            status: Status::None,
        }
    }
}

impl Card for MajorExplosion {
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
        let valid_zones = caster.get_zones_within_steps(state, 2);
        vec![
            Effect::set_card_status(self.get_id(), Status::SelectingZone),
            Effect::select_zone(&self.get_owner_id(), valid_zones),
        ]
    }

    fn set_status(&mut self, _status: &Box<dyn std::any::Any>) -> anyhow::Result<()> {
        let status = _status
            .downcast_ref::<Status>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast status"))?;
        self.status = status.clone();
        Ok(())
    }
}

impl MessageHandler for MajorExplosion {
    fn handle_message(&mut self, message: &ClientMessage, state: &State) -> Vec<Effect> {
        if message.player_id() != self.get_owner_id() {
            return vec![];
        }

        match (&self.status, message) {
            (Status::SelectingZone, ClientMessage::PickSquare { square, .. }) => {
                let zone = Zone::Realm(*square);
                let zone_dmg: Vec<(Option<Zone>, u8)> = vec![
                    (Some(zone.clone()), 7),
                    (zone.zone_in_direction(&Direction::Up), 5),
                    (zone.zone_in_direction(&Direction::Down), 5),
                    (zone.zone_in_direction(&Direction::Left), 5),
                    (zone.zone_in_direction(&Direction::Right), 5),
                    (zone.zone_in_direction(&Direction::TopLeft), 3),
                    (zone.zone_in_direction(&Direction::TopRight), 3),
                    (zone.zone_in_direction(&Direction::BottomLeft), 3),
                    (zone.zone_in_direction(&Direction::BottomRight), 3),
                ];

                let mut effects = vec![];
                for (zone, dmg) in zone_dmg {
                    if let Some(z) = zone {
                        let units = state.get_units_in_zone(&z);
                        for unit in units {
                            effects.push(Effect::take_damage(unit.get_id(), self.get_id(), dmg));
                        }
                    }
                }
                effects.push(Effect::set_card_status(self.get_id(), Status::None));
                effects.push(Effect::wait_for_play(self.get_id()));
                effects
            }
            _ => vec![],
        }
    }
}
