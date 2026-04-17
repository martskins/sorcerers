use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_card_with_preview},
    state::State,
};

#[derive(Debug, Clone)]
pub struct Browse {
    card_base: CardBase,
}

impl Browse {
    pub const NAME: &'static str = "Browse";
    pub const DESCRIPTION: &'static str = "Look at your next seven spells. Put one in your hand and the rest on the bottom of your spellbook in any order.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "AAA"),
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
impl Card for Browse {
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
        let deck = state.decks.get(&controller_id).unwrap().clone();

        // Take up to 7 cards from the top of the spellbook (last in vec = top of deck).
        let mut remaining_spells = deck.spells.clone();
        let mut looked_at: Vec<uuid::Uuid> = vec![];
        for _ in 0..7 {
            if let Some(card_id) = remaining_spells.pop() {
                looked_at.push(card_id);
            }
        }

        if looked_at.is_empty() {
            return Ok(vec![]);
        }

        // Player picks one card to go to hand.
        let chosen_id = pick_card_with_preview(
            &controller_id,
            &looked_at,
            state,
            "Browse: Pick a spell to put into your hand",
        )
        .await?;

        let mut bottom_spells: Vec<uuid::Uuid> = looked_at
            .iter()
            .filter(|id| **id != chosen_id)
            .cloned()
            .collect();

        // Player orders the remaining spells for the bottom of the deck (position 0).
        // Ask player to arrange them one by one from bottom to top.
        let mut ordered_bottom: Vec<uuid::Uuid> = vec![];
        while !bottom_spells.is_empty() {
            let position_label = if bottom_spells.len() == 1 {
                "the bottom".to_string()
            } else {
                format!("position {} from the bottom", ordered_bottom.len() + 1)
            };
            let picked_id = pick_card_with_preview(
                &controller_id,
                &bottom_spells,
                state,
                &format!("Browse: Pick a spell to place at {}", position_label),
            )
            .await?;
            ordered_bottom.push(picked_id);
            bottom_spells.retain(|id| id != &picked_id);
        }

        // Build the new spells order: ordered_bottom (index 0 = absolute bottom) + remaining_spells + chosen card on top temporarily removed.
        // ordered_bottom goes at the front of remaining_spells.
        let mut new_spells = ordered_bottom;
        new_spells.extend(remaining_spells);

        let effects = vec![
            Effect::RearrangeDeck {
                spells: new_spells,
                sites: deck.sites.clone(),
            },
            Effect::SetCardZone {
                card_id: chosen_id,
                zone: Zone::Hand,
            },
        ];

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Browse::NAME, |owner_id: PlayerId| {
    Box::new(Browse::new(owner_id))
});
