use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{Counter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Shrink {
    card_base: CardBase,
}

impl Shrink {
    pub const NAME: &'static str = "Shrink";
    pub const DESCRIPTION: &'static str =
        "Set the base power of target nearby unit to 0 until your next turn.";

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

        let base_power = state
            .get_card(&target_id)
            .get_unit_base()
            .map(|ub| ub.power as i16)
            .unwrap_or(0);

        Ok(vec![Effect::AddCounter {
            card_id: target_id,
            counter: Counter {
                id: uuid::Uuid::new_v4(),
                power: -base_power,
                toughness: 0,
                expires_on_effect: Some(EffectQuery::TurnStart {
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
