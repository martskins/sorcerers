use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{Effect, TokenType},
    game::PlayerId,
    state::State,
};

#[derive(Debug, Clone)]
pub struct StarSeedsOfUhr {
    card_base: CardBase,
}

impl StarSeedsOfUhr {
    pub const NAME: &'static str = "Star-seeds of Uhr";
    pub const DESCRIPTION: &'static str = "Fill all empty sites with Rubble tokens.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "E"),
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
impl Card for StarSeedsOfUhr {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let mut effects = vec![];
        for i in 0..=12u8 {
            let zone = Zone::Realm(i);
            if zone.get_site(state).is_none() {
                effects.push(Effect::SummonToken {
                    player_id: controller_id,
                    token_type: TokenType::Rubble,
                    zone,
                });
            }
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (StarSeedsOfUhr::NAME, |owner_id: PlayerId| {
    Box::new(StarSeedsOfUhr::new(owner_id))
});
