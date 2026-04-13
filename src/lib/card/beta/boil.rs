use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Boil {
    pub card_base: CardBase,
}

impl Boil {
    pub const NAME: &'static str = "Boil";
    pub const DESCRIPTION: &'static str =
        "Destroy all minions occupying target water site up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
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
impl Card for Boil {
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
        let controller_id = self.get_controller_id(state);
        let card = state.get_card(caster_id);
        let zones = card.get_zones_within_steps(state, 2);
        let Some(picked_site_id) = CardQuery::new()
            .with_element(Element::Water)
            .in_zones(&zones)
            .sites()
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let site = state.get_card(&picked_site_id);
        Ok(CardQuery::new()
            .in_zone(site.get_zone())
            .minions()
            .all(state)
            .into_iter()
            .map(|minion_id| Effect::BuryCard { card_id: minion_id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Boil::NAME, |owner_id: PlayerId| {
        Box::new(Boil::new(owner_id))
    });
