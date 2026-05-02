use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct SpinAttack {
    card_base: CardBase,
}

impl SpinAttack {
    pub const NAME: &'static str = "Spin Attack";
    pub const DESCRIPTION: &'static str = "An allied minion strikes each enemy at its location.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "F"),
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
impl Card for SpinAttack {
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
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let ally_id = match CardQuery::new()
            .units()
            .in_play()
            .controlled_by(&controller_id)
            .with_prompt("Spin Attack: Choose an ally to strike all enemies at its location")
            .pick(&controller_id, state, false)
            .await?
        {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        let ally = state.get_card(&ally_id);
        let ally_zone = ally.get_zone().clone();
        let ally_power = ally.get_unit_base().map(|ub| ub.power as u16).unwrap_or(0);
        let enemies: Vec<Effect> = CardQuery::new()
            .units()
            .in_zone(&ally_zone)
            .all(state)
            .into_iter()
            .filter(|&id| {
                let card = state.get_card(&id);
                card.get_controller_id(state) != controller_id
            })
            .map(|enemy_id| Effect::TakeDamage {
                card_id: enemy_id,
                from: ally_id,
                damage: ally_power,
                is_strike: true,
                is_ranged: false,
            })
            .collect();
        Ok(enemies)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SpinAttack::NAME, |owner_id: PlayerId| {
    Box::new(SpinAttack::new(owner_id))
});
