use crate::{
    card::{Card, CardBase, Cost, Edition, Plane, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    query::CardQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct RaiseDead {
    pub card_base: CardBase,
}

impl RaiseDead {
    pub const NAME: &'static str = "Raise Dead";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "AA"),
                plane: Plane::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RaiseDead {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let prompt = "Summon a random dead minion".to_string();
        let query = CardQuery::RandomUnitInZone {
            id: uuid::Uuid::new_v4(),
            zone: Zone::Cemetery,
        };
        let unit_id = query.resolve(self.get_owner_id(), state).await?;
        let unit = state.get_card(&unit_id);
        let zones = unit.get_valid_play_zones(state)?;
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, &prompt).await?;
        Ok(vec![Effect::SummonCard {
            player_id: self.get_owner_id().clone(),
            card_id: unit_id,
            zone: picked_zone,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (RaiseDead::NAME, |owner_id: PlayerId| Box::new(RaiseDead::new(owner_id)));
