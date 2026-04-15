use rand::seq::SliceRandom;

use crate::{
    card::{Card, CardBase, Cost, CostType, Costs, Edition, MinionType, Rarity, Zone},
    effect::Effect,
    game::{PlayerId, pick_card_with_options, reveal_cards, yes_or_no},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct AssortedAnimals {
    card_base: CardBase,
}

impl AssortedAnimals {
    pub const NAME: &'static str = "Assorted Animals";
    pub const DESCRIPTION: &'static str = "Search your spellbook for different Beasts with a combined mana cost of X or less, reveal them, and put them in your hand. Shuffle your spellbook.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::single(Cost::from_variable_mana("EE")),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for AssortedAnimals {
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
        cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?.clone();

        let x_cost = cost_paid
            .into_iter()
            .find_map(|cost| match cost {
                CostType::ManaCost(mana_paid) => Some(mana_paid),
                _ => None,
            })
            .unwrap_or_default();

        let mut beasts = CardQuery::new()
            .in_zone(&Zone::Spellbook)
            .minions()
            .minion_types(vec![MinionType::Beast])
            .mana_cost_less_than_or_equal_to(x_cost)
            .controlled_by(&controller_id)
            .all(state)
            .into_iter()
            .map(|id| {
                let card = state
                    .cards
                    .iter()
                    .find(|c| c.get_id() == &id)
                    .expect("card id to be valid");
                (
                    card.get_name().to_string(),
                    card.get_id().clone(),
                    card.get_base().costs.mana_value(),
                )
            })
            .collect::<Vec<_>>();

        beasts.sort_by(|a, b| a.0.cmp(&b.0));

        let mut remaining_mana = x_cost;
        let mut chosen = Vec::new();

        let mut display_card_ids: Vec<uuid::Uuid> = CardQuery::new()
            .controlled_by(&controller_id)
            .in_zone(&Zone::Spellbook)
            .all(state);
        loop {
            let affordable: Vec<_> = beasts
                .iter()
                .filter(|(_, _, cost)| *cost <= remaining_mana)
                .cloned()
                .collect();
            if affordable.is_empty() {
                break;
            }

            if !chosen.is_empty() {
                if !yes_or_no(
                    &controller_id,
                    state,
                    "Assorted Animals: Search for another Beast?",
                )
                .await?
                {
                    break;
                }
            }

            let picked_id = pick_card_with_options(
                &controller_id,
                &display_card_ids,
                &affordable
                    .iter()
                    .map(|(_, id, _)| id.clone())
                    .collect::<Vec<_>>(),
                false,
                state,
                "Assorted Animals: Pick a Beast to put into your hand",
            )
            .await?;

            let picked = beasts
                .iter()
                .find(|(_, id, _)| id == &picked_id)
                .cloned()
                .expect("picked beast to be present");
            remaining_mana = remaining_mana.saturating_sub(picked.2);
            chosen.push(picked);
            beasts.retain(|(_, id, _)| id != &picked_id);
            display_card_ids.retain(|id| id != &picked_id);
        }

        let chosen_ids: Vec<uuid::Uuid> = chosen.iter().map(|(_, id, _)| id.clone()).collect();
        if !chosen_ids.is_empty() {
            reveal_cards(
                &controller_id,
                &chosen_ids,
                state,
                "Assorted Animals: Revealed Beasts",
            )
            .await?;
        }

        let mut spells = deck.spells.clone();
        spells.retain(|id| !chosen_ids.contains(id));
        spells.shuffle(&mut rand::rng());

        let mut effects = vec![Effect::RearrangeDeck {
            spells,
            sites: deck.sites.clone(),
        }];

        effects.extend(chosen_ids.iter().map(|card_id| Effect::SetCardZone {
            card_id: card_id.clone(),
            zone: Zone::Hand,
        }));

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AssortedAnimals::NAME, |owner_id: PlayerId| {
        Box::new(AssortedAnimals::new(owner_id))
    });
