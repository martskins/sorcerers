use crate::{
    card::{Card, CardBase, CardType, Edition, MessageHandler, Modifier, UnitBase, Zone},
    effect::{Counter, Effect},
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct AskelonPhoenix {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AskelonPhoenix {
    pub const NAME: &'static str = "Askelon Phoenix";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                modifiers: vec![Modifier::Airborne],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 5,
                required_thresholds: Thresholds::parse("FF"),
            },
        }
    }
}

impl Card for AskelonPhoenix {
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

    fn on_take_damage(&mut self, state: &State, from: &uuid::Uuid, damage: u8) -> Vec<Effect> {
        let attacker = state.get_card(from).unwrap();
        if attacker.get_elements(state).contains(&Element::Fire) {
            return vec![Effect::AddCounter {
                card_id: self.get_id().clone(),
                counter: Counter::new(1, 1, Some(1)),
            }];
        }

        if let Some(ub) = self.get_unit_base_mut() {
            ub.damage += damage;
        }

        let mut effects = vec![];
        if attacker.has_modifier(state, Modifier::Lethal) {
            effects.push(Effect::BuryCard {
                card_id: self.get_id().clone(),
            });
        }
        effects
    }
}

impl MessageHandler for AskelonPhoenix {}
