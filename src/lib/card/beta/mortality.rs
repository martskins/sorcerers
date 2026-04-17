use crate::{
    card::{
        Card, CardBase, CardConstructor, CardType, Cost, Costs, Edition, MinionType, Rarity, Zone,
    },
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Mortality {
    card_base: CardBase,
}

impl Mortality {
    pub const NAME: &'static str = "Mortality";
    pub const DESCRIPTION: &'static str =
        "Kill all Mortal minions at target location up to two steps away.";

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
impl Card for Mortality {
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
        let caster_card = state.get_card(caster_id);
        let zones = caster_card.get_zones_within_steps(state, 2);
        let target_zone = pick_zone(
            &controller_id,
            &zones,
            state,
            false,
            "Mortality: Pick a location up to 2 steps away",
        )
        .await?;
        let effects = CardQuery::new()
            .card_types(vec![CardType::Minion])
            .minion_types(vec![MinionType::Mortal])
            .in_zone(&target_zone)
            .all(state)
            .iter()
            .map(|minion_id| Effect::BuryCard {
                card_id: *minion_id,
            })
            .collect();
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Mortality::NAME, |owner_id: PlayerId| {
    Box::new(Mortality::new(owner_id))
});
