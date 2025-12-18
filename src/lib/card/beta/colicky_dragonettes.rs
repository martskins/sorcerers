use crate::{
    card::{Card, CardBase, Edition, MessageHandler, Plane, UnitBase, Zone},
    effect::Effect,
    game::{CARDINAL_DIRECTIONS, InputStatus, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct ColickyDragonettes {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
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
            Effect::set_input_status(InputStatus::ShootingProjectile {
                player_id: self.get_owner_id().clone(),
                card_id: self.get_id().clone(),
                caster_id: Some(self.get_id().clone()),
                from: self.get_zone().clone(),
                direction: None,
                damage: 1,
                piercing: false,
            }),
            Effect::select_direction(self.get_owner_id(), &CARDINAL_DIRECTIONS),
        ]
    }
}

impl MessageHandler for ColickyDragonettes {}
