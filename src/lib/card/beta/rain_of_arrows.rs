use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct RainOfArrows {
    pub card_base: CardBase,
}

impl RainOfArrows {
    pub const NAME: &'static str = "Rain of Arrows";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "A"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for RainOfArrows {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        let minion_ids = CardMatcher::new()
            .with_card_type(CardType::Minion)
            .in_region(&Region::Surface)
            .resolve_ids(state);
        for minion_id in minion_ids {
            effects.push(Effect::take_damage(&minion_id, caster_id, 1));
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (RainOfArrows::NAME, |owner_id: PlayerId| {
    Box::new(RainOfArrows::new(owner_id))
});
