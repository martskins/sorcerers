use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FlameWave {
    card_base: CardBase,
}

impl FlameWave {
    pub const NAME: &'static str = "Flame Wave";
    pub const DESCRIPTION: &'static str = "Flame Wave flows horizontally, from one edge of the realm to the other. Deal damage to each unit atop sites in the area of effect:";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(6, "FF"),
                rarity: crate::card::Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for FlameWave {
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
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let spell_id = *self.get_id();
        let all_units = CardQuery::new().units().in_play().all(state);
        // TODO: This is incorrect, it should deal damage to units based on their position in the
        // realm, according to the following pattern:
        // 7|5|3|1
        // 7|5|3|1
        // 7|5|3|1
        // 7|5|3|1
        //
        // The player picks whether where the wave starts and whether to deal damage from left to
        // right or right to left. That is, damage always starts with a 7, and it can decrease to
        // the right or the left.
        let effects = all_units
            .into_iter()
            .map(|unit_id| Effect::TakeDamage {
                card_id: unit_id,
                from: spell_id,
                damage: 3,
                is_strike: false,
                is_ranged: false,
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (FlameWave::NAME, |owner_id: PlayerId| {
    Box::new(FlameWave::new(owner_id))
});
