use crate::{
    card::{Card, CardBase, CardType, Cost, Edition, MinionType, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_zone},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct Mortality {
    pub card_base: CardBase,
}

impl Mortality {
    pub const NAME: &'static str = "Mortality";

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
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Mortality {
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
        let minion_ids = CardMatcher::new()
            .with_card_types(vec![CardType::Minion])
            .with_minion_types(vec![MinionType::Mortal])
            .in_zone(&target_zone)
            .resolve_ids(state);
        let mut effects = Vec::new();
        for minion_id in minion_ids {
            effects.push(Effect::BuryCard { card_id: minion_id });
        }
        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Mortality::NAME, |owner_id: PlayerId| Box::new(Mortality::new(owner_id)));
