use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId, pick_card},
    query::ZoneQuery,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Riptide {
    pub card_base: CardBase,
}

impl Riptide {
    pub const NAME: &'static str = "Riptide";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "W"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Riptide {
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
        let water_sites = CardMatcher::new()
            .with_affinity(Element::Water)
            .card_type(CardType::Site)
            .resolve_ids(state);
        let prompt = "Riptide: Pick a water site to pull a unit into";
        let site_id = pick_card(self.get_controller_id(state), &water_sites, state, prompt).await?;
        let site = state.get_card(&site_id);
        let units = CardMatcher::units_adjacent(site.get_zone())
            .in_regions(vec![Region::Surface])
            .resolve_ids(state);
        if units.is_empty() {
            return Ok(vec![]);
        }

        let unit_id = pick_card(self.get_controller_id(state), &units, state, "Pick a unit to pull").await?;
        let unit = state.get_card(&unit_id);
        Ok(vec![
            Effect::MoveCard {
                player_id: self.get_controller_id(state),
                card_id: unit_id,
                from: unit.get_zone().clone(),
                to: ZoneQuery::from_zone(site.get_zone().clone()),
                tap: false,
                region: unit.get_region(state).clone(),
                through_path: None,
            },
            Effect::DrawCard {
                player_id: self.get_controller_id(state),
                count: 1,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Riptide::NAME, |owner_id: PlayerId| Box::new(Riptide::new(owner_id)));