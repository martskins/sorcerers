use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, MinionType, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_card_with_options},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct CallToWar {
    card_base: CardBase,
}

impl CallToWar {
    pub const NAME: &'static str = "Call to War";
    pub const DESCRIPTION: &'static str = "Search your spellbook for an Exceptional Mortal, reveal it, and put it into your hand. Shuffle your spellbook.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for CallToWar {
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
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.decks.get(&controller_id).unwrap();
        let targets = CardQuery::new()
            .minions()
            .minion_type(&MinionType::Mortal)
            .rarity(&Rarity::Exceptional)
            .all(state);
        if targets.is_empty() {
            return Ok(vec![Effect::ShuffleDeck {
                player_id: controller_id,
            }]);
        }

        let chosen = pick_card_with_options(
            &controller_id,
            &targets,
            &deck.spells,
            false,
            state,
            "Call to War: Choose an Exceptional Mortal to put into your hand",
        )
        .await?;

        Ok(vec![
            Effect::SetCardZone {
                card_id: chosen,
                zone: Zone::Hand,
            },
            Effect::ShuffleDeck {
                player_id: controller_id,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (CallToWar::NAME, |owner_id: PlayerId| {
    Box::new(CallToWar::new(owner_id))
});
