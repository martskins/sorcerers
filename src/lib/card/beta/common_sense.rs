use crate::{
    card::{Card, CardBase, Cost, Costs, Edition, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card_with_options},
    state::State,
};

#[derive(Debug, Clone)]
pub struct CommonSense {
    pub card_base: CardBase,
}

impl CommonSense {
    pub const NAME: &'static str = "Common Sense";
    pub const DESCRIPTION: &'static str =
        "Search your spellbook for an Ordinary card, reveal it, and put it into your hand. Shuffle your spellbook.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "E"),
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
impl Card for CommonSense {
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

        let targets: Vec<uuid::Uuid> = deck
            .spells
            .iter()
            .filter(|id| state.get_card(id).get_base().rarity == Rarity::Ordinary)
            .cloned()
            .collect();

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
            "Common Sense: Choose an Ordinary card to put into your hand",
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (CommonSense::NAME, |owner_id: PlayerId| {
    Box::new(CommonSense::new(owner_id))
});
