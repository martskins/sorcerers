use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct CallToWar {
    card_base: CardBase,
}

impl CallToWar {
    pub const NAME: &'static str = "Call to War";
    pub const DESCRIPTION: &'static str = "Search your spellbook for an Exceptional Mortal, reveal it, and put it into your hand. Shuffle your spellbook.";

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
impl Card for CallToWar {
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
impl Magic for CallToWar {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(picked_card_id) = CardQuery::new()
            .minions()
            .in_zone(Zone::Spellbook)
            .owned_by(&controller_id)
            .minion_type(&MinionType::Mortal)
            .rarity(&Rarity::Exceptional)
            .with_source_card(*self.get_id())
            .with_prompt("Pick an exceptional mortal to put in your hand")
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![
            Effect::SetCardZone {
                card_id: picked_card_id,
                zone: Zone::Hand,
            },
            Effect::ShuffleDeck {
                player_id: controller_id,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (CallToWar::NAME, |owner_id: PlayerId| {
    Box::new(CallToWar::new(owner_id))
});
