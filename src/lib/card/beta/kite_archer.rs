use crate::{
    card::{Card, CardBase, Edition, MinionType, Modifier, Plane, Rarity, UnitBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_option, pick_zone},
    query::ZoneQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct KiteArcher {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl KiteArcher {
    pub const NAME: &'static str = "Kite Archer";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                modifiers: vec![Modifier::Ranged(1)],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 3,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for KiteArcher {
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

    async fn after_attack(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let actions = vec!["Yes".to_string(), "No".to_string()];
        let picked_action = pick_option(self.get_owner_id(), &actions, state, "Take Step").await?;
        if actions[picked_action] == "No" {
            return Ok(vec![]);
        }

        let zones = self.get_zone().get_adjacent();
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, "Choose to step to").await?;
        Ok(vec![Effect::MoveCard {
            player_id: self.get_owner_id().clone(),
            card_id: self.get_id().clone(),
            from: self.get_zone().clone(),
            to: ZoneQuery::Specific {
                id: uuid::Uuid::new_v4(),
                zone: picked_zone.clone(),
            },
            tap: false,
            plane: self.card_base.plane.clone(),
            through_path: None,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (KiteArcher::NAME, |owner_id: PlayerId| {
    Box::new(KiteArcher::new(owner_id))
});
