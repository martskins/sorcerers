use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State},
    card::Ability,
};

#[derive(Debug, Clone)]
pub struct Shrink {
    card_base: CardBase,
}

impl Shrink {
    pub const NAME: &'static str = "Shrink";
    pub const DESCRIPTION: &'static str =
        "Target nearby unit is Disabled until the start of your next turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "W"),
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
impl Card for Shrink {
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
        let Some(target_id) = CardQuery::new()
            .units()
            .near_to(caster.get_zone())
            .with_prompt("Shrink: Pick a target unit")
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
                expires_on_effect: Some(EffectQuery::TurnEnd {
                    player_id: Some(controller_id),
                }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Shrink::NAME, |owner_id: PlayerId| {
    Box::new(Shrink::new(owner_id))
});
