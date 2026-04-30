use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct ScorchedEarth {
    card_base: CardBase,
}

impl ScorchedEarth {
    pub const NAME: &'static str = "Scorched Earth";
    pub const DESCRIPTION: &'static str = "Destroy a site and everything at that location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for ScorchedEarth {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let site_id = match CardQuery::new()
            .sites()
            .in_play()
            .with_prompt("Scorched Earth: Choose a site to destroy")
            .pick(&controller_id, state, false)
            .await?
        {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        let site_zone = state.get_card(&site_id).get_zone().clone();
        let mut effects = vec![Effect::BuryCard { card_id: site_id }];
        let occupants = CardQuery::new()
            .units()
            .in_zone(&site_zone)
            .all(state);
        for card_id in occupants {
            effects.push(Effect::BuryCard { card_id });
        }
        let artifacts = CardQuery::new()
            .artifacts()
            .in_zone(&site_zone)
            .all(state);
        for card_id in artifacts {
            effects.push(Effect::BuryCard { card_id });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ScorchedEarth::NAME, |owner_id: PlayerId| {
    Box::new(ScorchedEarth::new(owner_id))
});
