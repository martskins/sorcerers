use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MinecartMadness {
    card_base: CardBase,
}

impl MinecartMadness {
    pub const NAME: &'static str = "Minecart Madness";
    pub const DESCRIPTION: &'static str = "This turn, your units can move between any sites in a chosen span of land as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MinecartMadness {
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
        _state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        // TODO: add temporary movement adjacency for a chosen span of land.
        Ok(vec![])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MinecartMadness::NAME, |owner_id: PlayerId| {
        Box::new(MinecartMadness::new(owner_id))
    });
