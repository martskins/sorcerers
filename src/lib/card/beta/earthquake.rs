use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Earthquake {
    card_base: CardBase,
}

impl Earthquake {
    pub const NAME: &'static str = "Earthquake";
    pub const DESCRIPTION: &'static str = "You may rearrange sites within a two-by-two area, carrying along everything of normal size. Then burrow all minions and artifacts on those sites.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "EE"),
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
impl Card for Earthquake {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Earthquake {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let areas = Zone::all_intersections()
            .into_iter()
            .filter_map(Zone::into_location)
            .collect::<Vec<_>>();
        let area = pick_location(
            &controller_id,
            &areas,
            state,
            false,
            "Earthquake: Pick a two-by-two area",
        )
        .await?;
        let Location::Intersection(squares, _) = area else {
            return Ok(vec![]);
        };
        let affected_zones = squares
            .into_iter()
            .map(|square| Zone::Location(Location::Square(square, Region::Surface)))
            .collect::<Vec<Zone>>();
        let affected_cards = CardQuery::new()
            .card_types(vec![CardType::Minion, CardType::Artifact])
            .in_zones(&affected_zones)
            .normal_sized()
            .all(state);

        Ok(affected_cards
            .into_iter()
            .map(|card_id| Effect::SetCardRegion {
                card_id,
                destination: Region::Underground,
                tap: false,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Earthquake::NAME, |owner_id: PlayerId| {
    Box::new(Earthquake::new(owner_id))
});
