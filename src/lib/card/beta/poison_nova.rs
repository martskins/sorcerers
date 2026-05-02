use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

/// **Poison Nova** — Exceptional Magic (5 cost, F threshold)
///
/// Lethal. Deal 1 damage to each other nearby minion.
/// (Lethal means damage from this causes death regardless of toughness.)
#[derive(Debug, Clone)]
pub struct PoisonNova {
    card_base: CardBase,
}

impl PoisonNova {
    pub const NAME: &'static str = "Poison Nova";
    pub const DESCRIPTION: &'static str = "Lethal\n\nDeal 1 damage to each other nearby minion.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "F"),
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
impl Card for PoisonNova {
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
        let caster_zone = state.get_card(caster_id).get_zone().clone();

        // Deal 1 lethal damage to all minions nearby (adjacent sites).
        let nearby_minions = CardQuery::new().minions().near_to(&caster_zone).all(state);

        Ok(nearby_minions
            .into_iter()
            .flat_map(|card_id| {
                vec![
                    Effect::TakeDamage {
                        card_id,
                        from: *caster_id,
                        damage: 1,
                        is_strike: false,
                        is_ranged: false,
                    },
                    Effect::KillMinion {
                        card_id,
                        killer_id: *caster_id,
                    },
                ]
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PoisonNova::NAME, |owner_id: PlayerId| {
    Box::new(PoisonNova::new(owner_id))
});
