use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Unravel {
    card_base: CardBase,
}

impl Unravel {
    pub const NAME: &'static str = "Unravel";
    pub const DESCRIPTION: &'static str =
        "Destroy all artifacts and Undead minions at a location up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for Unravel {
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

        let picked_zone = pick_zone(
            &controller_id,
            &zones,
            state,
            false,
            "Unravel: Pick a location up to two steps away",
        )
        .await?;

        let artifacts = CardQuery::new()
            .artifacts()
            .in_zone(&picked_zone)
            .all(state);
        let undead = CardQuery::new()
            .minions()
            .minion_type(&MinionType::Undead)
            .in_zone(&picked_zone)
            .all(state);

        let mut effects: Vec<Effect> = artifacts
            .into_iter()
            .map(|card_id| Effect::BuryCard { card_id })
            .collect();
        effects.extend(
            undead
                .into_iter()
                .map(|card_id| Effect::BuryCard { card_id }),
        );

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Unravel::NAME, |owner_id: PlayerId| {
    Box::new(Unravel::new(owner_id))
});
