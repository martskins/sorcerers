use crate::{
    card::{Ability, Card, CardBase, CardType, Edition, MessageHandler, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct WayfaringPilgrim {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
    corners_entered: Vec<u8>,
}

impl WayfaringPilgrim {
    pub const NAME: &'static str = "Wayfaring Pilgrim";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("F"),
            },
            corners_entered: Vec::new(),
        }
    }
}

impl Card for WayfaringPilgrim {
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

    fn on_move(&mut self, state: &State, zone: &Zone) -> Vec<Effect> {
        let mut effects = Vec::new();
        match zone {
            Zone::Realm(s) => {
                if !self.corners_entered.contains(&s) {
                    self.corners_entered.push(*s);
                    effects.push(Effect::wait_for_card_draw(self.get_owner_id()));
                }
            }
            _ => {}
        }
        effects
    }
}

impl MessageHandler for WayfaringPilgrim {}
