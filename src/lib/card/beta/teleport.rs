use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds},
    query::{CardQuery, ZoneQuery},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Teleport {
    pub card_base: CardBase,
}

impl Teleport {
    pub const NAME: &'static str = "Teleport";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 2,
                required_thresholds: Thresholds::parse("AA"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Teleport {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, _state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::TeleportUnitToZone {
            player_id: self.get_owner_id().clone(),
            unit_query: CardQuery::OwnedBy {
                id: uuid::Uuid::new_v4(),
                owner: self.get_owner_id().clone(),
                prompt: Some("Teleport: Choose an ally to teleport".to_string()),
            },
            zone_query: ZoneQuery::AnySite {
                id: uuid::Uuid::new_v4(),
                controlled_by: None,
                prompt: Some("Teleport: Choose site to teleport to".to_string()),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Teleport::NAME, |owner_id: PlayerId| Box::new(Teleport::new(owner_id)));
