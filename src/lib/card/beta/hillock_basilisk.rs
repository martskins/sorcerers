use crate::{
    card::{Card, CardBase, Edition, Modifier, Plane, UnitBase, Zone},
    effect::{Counter, Effect},
    game::{Element, PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct HillockBasilisk {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl HillockBasilisk {
    pub const NAME: &'static str = "Hillock Basilisk";

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
                mana_cost: 4,
                required_thresholds: Thresholds::parse("F"),
                plane: Plane::Surface,
            },
        }
    }
}

impl Card for HillockBasilisk {
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

    fn area_modifiers(&self, state: &State) -> Vec<(Modifier, Vec<uuid::Uuid>)> {
        let nearby = self.get_zone().get_nearby();
        let units = nearby
            .iter()
            .flat_map(|z| state.get_units_in_zone(z))
            .map(|c| c.get_id().clone())
            .collect();
        vec![(Modifier::Disabled, units)]
    }
}
