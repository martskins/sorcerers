use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct CommonSense {
    card_base: CardBase,
}

impl CommonSense {
    pub const NAME: &'static str = "Common Sense";
    pub const DESCRIPTION: &'static str = "Search your spellbook for an Ordinary card, reveal it, and put it into your hand. Shuffle your spellbook.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for CommonSense {
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
impl Magic for CommonSense {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(chosen) = CardQuery::new()
            .owned_by(&controller_id)
            .in_zone(Zone::Spellbook)
            // TODO: This should show the rest of the cards in the spellbook, not just the eligbile
            // ones.
            .with_source_card(*self.get_id())
            .with_prompt("Choose an Ordinary card to put into your hand")
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![
            Effect::SetCardZone {
                card_id: chosen,
                zone: Zone::Hand,
            },
            Effect::ShuffleDeck {
                player_id: controller_id,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (CommonSense::NAME, |owner_id: PlayerId| {
    Box::new(CommonSense::new(owner_id))
});
