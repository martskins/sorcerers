use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct UnlikelyAlliance {
    card_base: CardBase,
}

impl UnlikelyAlliance {
    pub const NAME: &'static str = "Unlikely Alliance";
    pub const DESCRIPTION: &'static str =
        "Draw a card for each distinct rarity among your minions.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "A"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for UnlikelyAlliance {
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
        use std::collections::HashSet;
        let controller_id = state.get_card(caster_id).get_controller_id(state);
        let allied_minions = CardQuery::new()
            .minions()
            .in_play()
            .controlled_by(&controller_id)
            .all(state);
        let distinct_rarities: HashSet<String> = allied_minions
            .iter()
            .map(|id| {
                let card = state.get_card(id);
                format!("{:?}", card.get_base().rarity)
            })
            .collect();
        let count = distinct_rarities.len() as u8;
        if count == 0 {
            return Ok(vec![]);
        }
        Ok(vec![Effect::DrawCard {
            player_id: controller_id,
            count,
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (UnlikelyAlliance::NAME, |owner_id: PlayerId| {
        Box::new(UnlikelyAlliance::new(owner_id))
    });
