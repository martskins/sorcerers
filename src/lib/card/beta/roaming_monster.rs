use crate::{
    card::{Card, CardBase, Edition, MinionType, Plane, Rarity, UnitBase, Zone},
    game::{PlayerId, Thresholds},
    state::State,
};

#[derive(Debug, Clone)]
pub struct RoamingMonster {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl RoamingMonster {
    pub const NAME: &'static str = "Roaming Monster";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                modifiers: vec![],
                types: vec![MinionType::Monster],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 5,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Air,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RoamingMonster {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok((1..=20)
            .filter_map(
                |z| match state.get_cards_in_zone(&Zone::Realm(z)).iter().find(|c| c.is_site()) {
                    Some(_) => Some(Zone::Realm(z)),
                    None => None,
                },
            )
            .collect())
    }
}
