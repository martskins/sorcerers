use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BorderMilitia {
    card_base: CardBase,
}

impl BorderMilitia {
    pub const NAME: &'static str = "Border Militia";
    pub const DESCRIPTION: &'static str =
        "Summon a Foot Soldier token to each site you control that borders an enemy site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BorderMilitia {
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
impl Magic for BorderMilitia {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let enemy_site_locations = CardQuery::new()
            .sites()
            .not_controlled_by(&self.get_controller_id(state))
            .all_map(state, |card| card.get_location().clone());
        let locations = CardQuery::new()
            .sites()
            .controlled_by(&self.get_controller_id(state))
            .adjacent_locations_to_any(&enemy_site_locations)
            .all_map(state, |card| card.get_location().clone());

        Ok(locations
            .into_iter()
            .map(|location| Effect::SummonToken {
                player_id: self.get_controller_id(state),
                token_type: TokenType::FootSoldier,
                location,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (BorderMilitia::NAME, |owner_id: PlayerId| {
        Box::new(BorderMilitia::new(owner_id))
    });
