use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Sleep {
    card_base: CardBase,
}

impl Sleep {
    pub const NAME: &'static str = "Sleep";
    pub const DESCRIPTION: &'static str = "Target minion at a location up to two steps away falls asleep. It's disabled until it takes damage.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for Sleep {
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
impl Magic for Sleep {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let zones = caster.get_locations_within_steps(state, 2);
        let Some(target_id) = CardQuery::new()
            .minions()
            .in_locations(&zones)
            .with_prompt("Pick a target minion")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::AddStatusCounter {
            card_id: target_id,
            counter: StatusCounter {
                id: uuid::Uuid::new_v4(),
                status: CardStatus::Disabled,
                expires_on_effect: Some(EffectQuery::DamageDealt {
                    source: None,
                    target: Some(target_id.into()),
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Sleep::NAME, |owner_id: PlayerId| {
    Box::new(Sleep::new(owner_id))
});
