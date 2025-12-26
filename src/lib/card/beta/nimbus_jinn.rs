use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{Action, PlayerId, Thresholds, pick_card},
    state::State,
};

#[derive(Debug, Clone)]
enum NimbusJinnAction {
    DealDamage,
}

#[async_trait::async_trait]
impl Action for NimbusJinnAction {
    fn get_name(&self) -> &str {
        todo!()
    }

    async fn on_select(&self, card_id: Option<&uuid::Uuid>, player_id: &PlayerId, state: &State) -> Vec<Effect> {
        match self {
            NimbusJinnAction::DealDamage => {
                let card_id = card_id.unwrap();
                let card = state.get_card(card_id).unwrap();
                let units = state
                    .get_units_in_zone(card.get_zone())
                    .iter()
                    .filter(|c| c.get_id() != card_id)
                    .map(|c| c.get_id().clone())
                    .collect::<Vec<uuid::Uuid>>();
                if units.len() == 0 {
                    return vec![];
                }

                let picked_card = pick_card(player_id, &units, state, "Pick a unit to deal 3 damage to").await;
                vec![Effect::TakeDamage {
                    card_id: picked_card.clone(),
                    from: card_id.clone(),
                    damage: 3,
                }]
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct NimbusJinn {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl NimbusJinn {
    pub const NAME: &'static str = "Nimbus Jinn";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                modifiers: vec![Modifier::Airborne],
                types: vec![MinionType::Spirit],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 6,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Air,
                rarity: Rarity::Elite,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for NimbusJinn {
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

    fn get_actions(&self, state: &State) -> Vec<Box<dyn Action>> {
        let mut actions = self.base_unit_actions(state);
        actions.push(Box::new(NimbusJinnAction::DealDamage));
        actions
    }
}
