use rand::seq::SliceRandom;

use crate::{
    card::{Card, CardBase, CardType, Cost, CostType, Costs, Edition, MinionType, Rarity, Region, Zone},
    effect::Effect,
    game::{PlayerId, pick_card, reveal_cards, yes_or_no},
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct AssortedAnimals {
    pub card_base: CardBase,
}

impl AssortedAnimals {
    pub const NAME: &'static str = "Assorted Animals";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::from_cost(Cost::from_variable_mana("EE")),
                region: Region::Surface,
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

        let mut beasts = CardMatcher::new()
            .in_zone(&Zone::Spellbook)
            .with_card_type(CardType::Minion)
            .with_minion_types(vec![MinionType::Beast])
            .with_mana_cost_less_than_or_equal_to(x_cost)
            .with_controller_id(&controller_id)
            .resolve_ids(state)
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
                    card.get_base().costs.mana_cost(),
                )
            })
            .collect::<Vec<_>>();

        beasts.sort_by(|a, b| a.0.cmp(&b.0));
        beasts.dedup_by(|a, b| a.0 == b.0);

        let mut remaining_mana = x_cost;
        let mut chosen = Vec::new();

        loop {
            let affordable: Vec<_> = beasts
                .iter()
                .filter(|(_, _, cost)| *cost <= remaining_mana)
                .cloned()
                .collect();
            if affordable.is_empty() {
                break;
            }

            if !yes_or_no(&controller_id, state, "Assorted Animals: Search for another Beast?").await? {
                break;
            }

            let picked_id = pick_card(
                &controller_id,
                &affordable.iter().map(|(_, id, _)| id.clone()).collect::<Vec<_>>(),
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
        }

        let chosen_ids: Vec<uuid::Uuid> = chosen.iter().map(|(_, id, _)| id.clone()).collect();
        if !chosen_ids.is_empty() {
            reveal_cards(&controller_id, &chosen_ids, state, "Assorted Animals: Revealed Beasts").await?;
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
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (AssortedAnimals::NAME, |owner_id: PlayerId| {
    Box::new(AssortedAnimals::new(owner_id))
});
