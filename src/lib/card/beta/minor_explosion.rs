use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct MinorExplosion {
    card_base: CardBase,
}

impl MinorExplosion {
    pub const NAME: &'static str = "Minor Explosion";
    pub const DESCRIPTION: &'static str =
        "Deal 3 damage to each unit at target location up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
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
impl Card for MinorExplosion {
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
        let valid_zones = caster.get_zones_within_steps(state, 2);
        let prompt = "Pick a zone to center Minor Explosion:";
        let zone = pick_zone(self.get_owner_id(), &valid_zones, state, false, prompt).await?;
        Ok(CardQuery::new()
            .units()
            .in_zone(&zone)
            .all(state)
            .into_iter()
            .map(|id| Effect::TakeDamage {
                card_id: id,
                from: *caster_id,
                damage: 3,
                is_strike: false,
                is_ranged: false,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MinorExplosion::NAME, |owner_id: PlayerId| {
        Box::new(MinorExplosion::new(owner_id))
    });
