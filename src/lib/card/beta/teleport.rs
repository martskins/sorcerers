use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    query::ZoneQuery,
    state::{CardQuery, State},
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
                costs: Costs::basic(2, "AA"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![Effect::TeleportUnitToZone {
            player_id: self.get_owner_id().clone(),
            unit_query: CardQuery::new()
                .count(1)
                .with_prompt("Teleport: Choose an ally to teleport")
                .units()
                .controlled_by(&self.get_controller_id(state)),
            zone_query: ZoneQuery::any_site(None, Some("Teleport: Choose site to teleport to".to_string())),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Teleport::NAME, |owner_id: PlayerId| Box::new(Teleport::new(owner_id)));
