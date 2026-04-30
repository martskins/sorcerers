use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{Effect, TokenType},
    game::PlayerId,
    state::State,
};

/// **Plague of Frogs** — Unique Magic (1 cost, WWW threshold)
///
/// Summon seven Frog tokens.
#[derive(Debug, Clone)]
pub struct PlagueOfFrogs {
    card_base: CardBase,
}

impl PlagueOfFrogs {
    pub const NAME: &'static str = "Plague of Frogs";
    pub const DESCRIPTION: &'static str = "Summon seven Frog tokens.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "WWW"),
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
impl Card for PlagueOfFrogs {
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
        let caster_zone = state.get_card(caster_id).get_zone().clone();

        Ok((0..7)
            .map(|_| Effect::SummonToken {
                player_id: controller_id,
                token_type: TokenType::Frog,
                zone: caster_zone.clone(),
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (PlagueOfFrogs::NAME, |owner_id: PlayerId| {
        Box::new(PlagueOfFrogs::new(owner_id))
    });
