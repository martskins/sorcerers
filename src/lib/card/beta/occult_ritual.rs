use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct OccultRitual {
    card_base: CardBase,
}

impl OccultRitual {
    pub const NAME: &'static str = "Occult Ritual";
    pub const DESCRIPTION: &'static str = "Gain ② this turn for each allied Spellcaster here.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for OccultRitual {
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
impl Magic for OccultRitual {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);

        let count = CardQuery::new()
            .in_location(caster.get_location().clone())
            .spellcasters(None)
            .controlled_by(&controller_id)
            .all(state)
            .len() as i8;
        if count == 0 {
            return Ok(vec![]);
        }

        Ok(vec![Effect::AdjustMana {
            player_id: controller_id,
            amount: count * 2,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (OccultRitual::NAME, |owner_id: PlayerId| {
    Box::new(OccultRitual::new(owner_id))
});
