use crate::prelude::*;
use rand::seq::SliceRandom;

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
                controller_id: owner_id,
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for AssortedAnimals {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let deck = state.get_player_deck(&controller_id)?.clone();

        let x_cost = cost_paid.payable_mana_value().unwrap_or_default();
        let mut beasts = CardQuery::new()
            .including_not_in_play()
            .in_zone(&Zone::Spellbook)
            .minions()
            .minion_types(vec![MinionType::Beast])
            .mana_cost_lte(x_cost)
            .controlled_by(&controller_id)
            .all(state)
            .into_iter()
            .map(|id| {
                let card = state.get_card(&id);
                (
                    card.get_name().to_string(),
                    *card.get_id(),
                    card.get_base()
                        .costs
                        .printed_mana_value()
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();

        beasts.sort_by(|a, b| a.0.cmp(&b.0));

        let mut remaining_mana = x_cost;
        let mut chosen = Vec::new();

        let mut display_card_ids: Vec<CardId> = CardQuery::new()
            .including_not_in_play()
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

            if !chosen.is_empty()
                && !yes_or_no_source(
                    &controller_id,
                    state,
                    "Search for another Beast?",
                    Some(*self.get_id()),
                )
                .await?
            {
                break;
            }

            let picked_id = pick_card_with_options(
                &controller_id,
                &display_card_ids,
                &affordable.iter().map(|(_, id, _)| *id).collect::<Vec<_>>(),
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

        let chosen_ids: Vec<CardId> = chosen.iter().map(|(_, id, _)| *id).collect();
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

        let mut effects = vec![];
        if chosen_ids.is_empty() {
            effects.push(Effect::Notify {
                message: "No valid cards to summon with Assorted Animals.".to_string(),
            });
        }

        effects.push(Effect::RearrangeDeck {
            spells,
            sites: deck.sites.clone(),
        });

        effects.extend(chosen_ids.iter().map(|card_id| Effect::SetCardZone {
            card_id: *card_id,
            zone: Zone::Hand,
        }));

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AssortedAnimals::NAME, |owner_id: PlayerId| {
        Box::new(AssortedAnimals::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::WildBoars;

    #[tokio::test]
    async fn reports_when_no_valid_beasts_exist() {
        let state = State::new_mock_state(Vec::new());
        let player_id = state.players[0].id;
        let assorted_animals = AssortedAnimals::new(player_id);

        let effects = assorted_animals
            .resolve_magic(&state, assorted_animals.get_id(), Cost::mana_only(3))
            .await
            .expect("Assorted Animals should resolve");

        assert!(matches!(
            effects.first(),
            Some(Effect::Notify { message }) if message == "No valid cards to summon with Assorted Animals."
        ));
    }

    #[test]
    fn query_finds_beasts_in_spellbook() {
        let mut state = State::new_mock_state(Vec::new());
        let player_id = state.players[0].id;

        let wild_boars = WildBoars::new(player_id);
        let wild_boars_id = *wild_boars.get_id();
        state.cards.insert(wild_boars_id, Box::new(wild_boars));

        let beasts = CardQuery::new()
            .including_not_in_play()
            .in_zone(&Zone::Spellbook)
            .minions()
            .minion_types(vec![MinionType::Beast])
            .mana_cost_lte(1)
            .controlled_by(&player_id)
            .all(&state);

        assert_eq!(beasts, vec![wild_boars_id]);
    }
}
