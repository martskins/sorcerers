use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Immolation {
    card_base: CardBase,
}

impl Immolation {
    pub const NAME: &'static str = "Immolation";
    pub const DESCRIPTION: &'static str = "Deal 7 damage to target minion nearby.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "FFF"),
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
impl Card for Immolation {
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
impl Magic for Immolation {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_location = caster.get_location().clone();

        let Some(target_id) = CardQuery::new()
            .minions()
            .near_to(&caster_location)
            .with_prompt("Pick target minion")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::TakeDamage {
            card_id: target_id,
            from: *self.get_id(),
            damage: Damage::basic(7),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Immolation::NAME, |owner_id: PlayerId| {
    Box::new(Immolation::new(owner_id))
});
