use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Exorcism {
    card_base: CardBase,
}

impl Exorcism {
    pub const NAME: &'static str = "Exorcism";
    pub const DESCRIPTION: &'static str =
        "Banish all Demon and Undead minions at target location up to two steps away.";

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
impl Card for Exorcism {
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
        let caster = state.get_card(caster_id);
        let zones = caster.get_zones_within_steps(state, 2);

        let picked_zone = pick_zone(
            self.get_owner_id(),
            &zones,
            state,
            false,
            "Exorcism: Pick a target location",
        )
        .await?;

        let targets = CardQuery::new()
            .minions()
            .minion_types(vec![MinionType::Demon, MinionType::Undead])
            .in_zone(&picked_zone)
            .all(state);

        Ok(targets
            .into_iter()
            .map(|card_id| Effect::BanishCard { card_id })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Exorcism::NAME, |owner_id: PlayerId| {
    Box::new(Exorcism::new(owner_id))
});
