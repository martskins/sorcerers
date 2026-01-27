use crate::{
    card::{Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
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
                cost: Cost::new(2, "A"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
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

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let zones = Zone::all_realm();
        let picked_zone = pick_zone(
            self.get_owner_id(),
            &zones,
            state,
            false,
            "Lightning Bolt: Choose a zone",
        )
        .await?;
        Ok(vec![Effect::DealDamageToTarget {
            player_id: self.get_owner_id().clone(),
            query: CardQuery::RandomUnitInZone {
                id: uuid::Uuid::new_v4(),
                zone: picked_zone,
            },
            from: caster_id.clone(),
            damage: 3,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (LightningBolt::NAME, |owner_id: PlayerId| {
    Box::new(LightningBolt::new(owner_id))
});
