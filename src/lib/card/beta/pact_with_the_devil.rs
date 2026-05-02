use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::State,
};

/// **Pact with the Devil** — Unique Magic (4 cost, FF threshold)
///
/// Sacrifice the caster or lose half your life, rounding up. If you do, draw three cards.
/// Implemented as: draw 3 cards and deal damage equal to half the caster's remaining HP (rounded up).
#[derive(Debug, Clone)]
pub struct PactWithTheDevil {
    card_base: CardBase,
}

impl PactWithTheDevil {
    pub const NAME: &'static str = "Pact with the Devil";
    pub const DESCRIPTION: &'static str =
        "Sacrifice the caster or lose half your life, rounding up. If you do, draw three cards.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for PactWithTheDevil {
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
        let avatar_id = state.get_player_avatar_id(&controller_id)?;
        let avatar = state.get_card(&avatar_id);

        // Calculate current HP: max toughness minus damage already taken.
        let max_hp = avatar.get_unit_base().map(|ub| ub.toughness).unwrap_or(20);
        let damage_taken = avatar.get_unit_base().map(|ub| ub.damage).unwrap_or(0);
        let current_hp = max_hp.saturating_sub(damage_taken);
        let half_hp = current_hp.div_ceil(2);

        Ok(vec![
            Effect::TakeDamage {
                card_id: avatar_id,
                from: *caster_id,
                damage: half_hp,
                is_strike: false,
                is_ranged: false,
            },
            Effect::DrawCard {
                player_id: controller_id,
                count: 3,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PactWithTheDevil::NAME, |owner_id: PlayerId| {
        Box::new(PactWithTheDevil::new(owner_id))
    });
