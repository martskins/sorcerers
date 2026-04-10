use crate::{
    card::{Card, CardBase, CardType, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId, pick_card},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Boil {
    pub card_base: CardBase,
}

impl Boil {
    pub const NAME: &'static str = "Boil";

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
        let card = state.get_card(caster_id);
        let zones = card.get_zones_within_steps(state, 2);
        let water_sites = CardMatcher::new()
            .with_element(Element::Water)
            .in_zones(&zones)
            .with_card_type(CardType::Site)
            .resolve_ids(state);
        if water_sites.len() == 0 {
            return Ok(vec![]);
        }

        let controller_id = card.get_controller_id(state);
        let picked_site_id = pick_card(&controller_id, &water_sites, state, "Choose a Water Site to destroy.").await?;
        let site = state.get_card(&picked_site_id);

        Ok(CardMatcher::new()
            .in_zone(site.get_zone())
            .with_card_type(CardType::Minion)
            .resolve_ids(state)
            .into_iter()
            .map(|minion_id| Effect::BuryCard { card_id: minion_id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Boil::NAME, |owner_id: PlayerId| Box::new(Boil::new(owner_id)));
