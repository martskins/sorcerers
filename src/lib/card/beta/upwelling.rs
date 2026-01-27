use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Upwelling {
    pub card_base: CardBase,
}

impl Upwelling {
    pub const NAME: &'static str = "Upwelling";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "WW"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Upwelling {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let nearby_sites = CardMatcher::sites_near(caster.get_zone()).resolve_ids(state);
        let prompt = "Upwelling: Pick a site";
        let site_id = pick_card(controller_id, &nearby_sites, state, prompt).await?;
        let site = state.get_card(&site_id);
        let cards = CardMatcher::new()
            .in_zone(site.get_zone())
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .resolve_ids(state);
        Ok(cards
            .into_iter()
            .map(|card_id| Effect::SetCardZone {
                card_id,
                zone: Zone::Hand,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Upwelling::NAME, |owner_id: PlayerId| Box::new(Upwelling::new(owner_id)));