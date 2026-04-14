use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{Element, PlayerId},
    query::ZoneQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Riptide {
    pub card_base: CardBase,
}

impl Riptide {
    pub const NAME: &'static str = "Riptide";
    pub const DESCRIPTION: &'static str =
        "Target water site pulls in an aboveground unit it's adjacent to. Draw a card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for Riptide {
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
        let Some(picked_site_id) = CardQuery::new()
            .with_affinity(Element::Water)
            .with_prompt("Riptide: Pick a water site to pull a unit into")
            .sites()
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let site = state.get_card(&picked_site_id);
        let Some(picked_unit_id) = CardQuery::new()
            .minions()
            .adjacent_to(site.get_zone())
            .in_regions(vec![Region::Surface])
            .with_prompt("Riptide: Pick a unit to pull")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let unit = state.get_card(&picked_unit_id);
        Ok(vec![
            Effect::MoveCard {
                player_id: self.get_controller_id(state),
                card_id: picked_unit_id,
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
    (Riptide::NAME, |owner_id: PlayerId| {
        Box::new(Riptide::new(owner_id))
    });
