use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct StormySeas {
    card_base: CardBase,
}

impl StormySeas {
    pub const NAME: &'static str = "Stormy Seas";
    pub const DESCRIPTION: &'static str =
        "Submerge all minions and artifacts occupying target water site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "W"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for StormySeas {
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
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        use crate::card::Region;
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let site_id = match CardQuery::new()
            .water_sites()
            .in_play()
            .with_prompt("Stormy Seas: Choose a water site")
            .pick(&controller_id, state, false)
            .await?
        {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        let site_zone = state.get_card(&site_id).get_zone().clone();
        let mut effects = vec![];
        let units = CardQuery::new().units().in_zone(&site_zone).all(state);
        for card_id in units {
            effects.push(Effect::SetCardRegion {
                card_id,
                region: Region::Underwater,
                tap: false,
            });
        }
        let artifacts = CardQuery::new().artifacts().in_zone(&site_zone).all(state);
        for card_id in artifacts {
            effects.push(Effect::SetCardRegion {
                card_id,
                region: Region::Underwater,
                tap: false,
            });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (StormySeas::NAME, |owner_id: PlayerId| {
    Box::new(StormySeas::new(owner_id))
});
