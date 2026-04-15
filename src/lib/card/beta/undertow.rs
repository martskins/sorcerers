use crate::{
    card::{Card, CardBase, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Undertow {
    pub site_base: SiteBase,
    pub card_base: CardBase,
}

impl Undertow {
    pub const NAME: &'static str = "Undertow";
    pub const DESCRIPTION: &'static str =
        "Genesis → Staying within this body of water, move target unit one step.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Undertow {}

#[async_trait::async_trait]
impl Card for Undertow {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    async fn genesis(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let player_id = self.get_controller_id(state);
        let body_of_water = state
            .get_body_of_water_at(self.get_zone())
            .ok_or(anyhow::anyhow!("Undertow must be in a body of water"))?;
        let controller_id = self.get_controller_id(state);
        let Some(unit_id) = CardQuery::new()
            .units()
            .with_prompt("Undertow: Choose a unit in the same body of water to move")
            .in_zones(&body_of_water)
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let unit = state.get_card(&unit_id);
        let zones = unit.get_zones_within_steps(state, 1);
        let picked_zone = pick_zone(
            player_id,
            &zones,
            state,
            false,
            "Undertow: Choose a zone to move the unit to",
        )
        .await?;
        Ok(vec![Effect::MoveCard {
            card_id: unit_id,
            to: ZoneQuery::from_zone(picked_zone),
            player_id,
            from: unit.get_zone().clone(),
            tap: false,
            region: unit.get_region(state).clone(),
            through_path: None,
        }])
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Undertow::NAME, |owner_id: PlayerId| {
        Box::new(Undertow::new(owner_id))
    });
