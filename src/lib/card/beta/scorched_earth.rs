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
    pub const DESCRIPTION: &'static str =
        "Choose any number of sites you control. Destroy each of those sites and everything there.";

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
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let mut remaining_sites = CardQuery::new()
            .sites()
            .in_play()
            .controlled_by(&controller_id)
            .all(state);
        let mut chosen_sites = vec![];

        loop {
            let Some(site_id) = CardQuery::from_ids(remaining_sites.clone())
                .with_prompt("Scorched Earth: Choose a site to destroy (or cancel)")
                .pick(&controller_id, state, false)
                .await?
            else {
                break;
            };

            chosen_sites.push(site_id);
            remaining_sites.retain(|id| *id != site_id);
            if remaining_sites.is_empty() {
                break;
            }
        }

        let mut effects = vec![];
        for site_id in chosen_sites {
            let site_zone = state.get_card(&site_id).get_zone().clone();
            effects.push(Effect::BuryCard { card_id: site_id });
            for card_id in CardQuery::new().units().in_zone(&site_zone).all(state) {
                effects.push(Effect::BuryCard { card_id });
            }
            for card_id in CardQuery::new().artifacts().in_zone(&site_zone).all(state) {
                effects.push(Effect::BuryCard { card_id });
            }
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (ScorchedEarth::NAME, |owner_id: PlayerId| {
        Box::new(ScorchedEarth::new(owner_id))
    });
