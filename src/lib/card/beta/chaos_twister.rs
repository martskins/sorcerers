use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card_with_preview},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct ChaosTwister {
    pub card_base: CardBase,
}

impl ChaosTwister {
    pub const NAME: &'static str = "Chaos Twister";
    pub const DESCRIPTION: &'static str = "Place target minion on the back of your hand, then blow it off from a height of at least one foot. Deal damage equal to its power to each unit atop the site it lands on, including itself.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "AA"),
                region: Region::Surface,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ChaosTwister {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_description(&self) -> &str {
        Self::DESCRIPTION
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
        let controller_id = self.get_controller_id(state);

        let all_minions = CardQuery::new().minions().all(state);
        if all_minions.is_empty() {
            return Ok(vec![]);
        }

        let target_id = pick_card_with_preview(
            &controller_id,
            &all_minions,
            state,
            "Chaos Twister: Choose a minion to fling",
        )
        .await?;

        let target = state.get_card(&target_id);
        let power = target
            .get_power(state)?
            .ok_or_else(|| anyhow::anyhow!("target has no power"))?;
        let from_zone = target.get_zone().clone();
        let region = target.get_region(state).clone();

        // Move the minion to a random site zone, then deal power damage to all units there.
        let landing_zone = ZoneQuery::from_options(Zone::all_realm(), None).randomised();

        Ok(vec![
            Effect::MoveCard {
                player_id: controller_id.clone(),
                card_id: target_id.clone(),
                from: from_zone,
                to: landing_zone.clone(),
                tap: false,
                region,
                through_path: None,
            },
            Effect::DealDamageAllUnitsInZone {
                player_id: controller_id,
                zone: landing_zone,
                from: self.get_id().clone(),
                damage: power,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (ChaosTwister::NAME, |owner_id: PlayerId| {
        Box::new(ChaosTwister::new(owner_id))
    });
