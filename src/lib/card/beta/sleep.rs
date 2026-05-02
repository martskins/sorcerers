use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State},
};

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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let zones = caster.get_zones_within_steps(state, 2);
        let Some(target_id) = CardQuery::new()
            .minions()
            .in_zones(&zones)
            .with_prompt("Sleep: Pick a target minion")
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![Effect::AddAbilityCounter {
            card_id: target_id,
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Disabled,
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
