use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LightningBolt {
    card_base: CardBase,
}

impl LightningBolt {
    pub const NAME: &'static str = "Lightning Bolt";
    pub const DESCRIPTION: &'static str = "Deal 3 damage to a random unit at target location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
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
impl Card for LightningBolt {
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
impl Magic for LightningBolt {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let caster_region = state.get_card(caster_id).get_region(state);
        let locations = Location::all_in_region(caster_region.clone());
        let picked_zone = pick_location(
            self.get_owner_id(),
            &locations,
            state,
            false,
            "Lightning Bolt: Choose a zone",
        )
        .await?;
        let Some(card_id) = CardQuery::new()
            .units()
            .in_zone(&picked_zone)
            .randomised()
            .count(1)
            .pick(&self.get_controller_id(state), state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::TakeDamage {
            card_id,
            from: *self.get_id(),
            damage: Damage::basic(3),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (LightningBolt::NAME, |owner_id: PlayerId| {
        Box::new(LightningBolt::new(owner_id))
    });
