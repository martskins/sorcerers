use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
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
                costs: Costs::basic(4, "AA"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
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

    async fn on_cast(
        &mut self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let prompt = "Summon a random dead minion".to_string();
        let minion_id = CardQuery::new()
            .count(1)
            .randomised()
            .in_zone(&Zone::Cemetery)
            .minions()
            .pick(self.get_owner_id(), state, false)
            .await?
            .expect("Raise Dead: No valid targets in cemetery");
        let minion = state.get_card(&minion_id);
        let zones = minion.get_valid_play_zones(state)?;
        let picked_zone = pick_zone(self.get_owner_id(), &zones, state, false, &prompt).await?;
        Ok(vec![Effect::SummonCard {
            player_id: self.get_owner_id().clone(),
            card_id: minion_id,
            zone: picked_zone,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (RaiseDead::NAME, |owner_id: PlayerId| Box::new(RaiseDead::new(owner_id)));
