use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct LightningBolt {
    pub card_base: CardBase,
}

impl LightningBolt {
    pub const NAME: &'static str = "Lightning Bolt";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("A"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for LightningBolt {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> Vec<Effect> {
        let zones = Zone::all_realm();
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, "Lightning Bolt: Choose a zone").await;
        vec![Effect::DealDamageToTarget {
            player_id: self.get_owner_id().clone(),
            query: CardQuery::RandomUnitInZone {
                id: uuid::Uuid::new_v4(),
                zone: picked_zone,
            },
            from: caster_id.clone(),
            damage: 3,
        }]
    }
}
