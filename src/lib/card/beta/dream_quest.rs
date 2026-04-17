use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::{Element, PlayerId, yes_or_no},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct DreamQuest {
    card_base: CardBase,
}

impl DreamQuest {
    pub const NAME: &'static str = "Dream-Quest";
    pub const DESCRIPTION: &'static str = "An allied Spellcaster falls asleep and is disabled until hurt. At the start of your next turn, if it's still asleep, you may wake it up to search your spellbook for a card and put it into your hand. Shuffle if needed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "A"),
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
impl Card for DreamQuest {
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

        // Find all allied Spellcaster minions in play.
        let all_minions = CardQuery::new()
            .controlled_by(&controller_id)
            .minions()
            .in_play()
            .all(state);
        let spellcasters: Vec<uuid::Uuid> = all_minions
            .into_iter()
            .filter(|id| {
                let card = state.get_card(id);
                [
                    Some(Element::Fire),
                    Some(Element::Water),
                    Some(Element::Earth),
                    Some(Element::Air),
                    None,
                ]
                .iter()
                .any(|e| card.has_ability(state, &Ability::Spellcaster(e.clone())))
            })
            .collect();

        if spellcasters.is_empty() {
            return Ok(vec![]);
        }

        let picked_id = CardQuery::from_ids(spellcasters)
            .with_prompt("Dream-Quest: Pick an allied Spellcaster to send on a dream quest")
            .pick(&controller_id, state, false)
            .await?;

        let Some(minion_id) = picked_id else {
            return Ok(vec![]);
        };

        let counter_id = uuid::Uuid::new_v4();

        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: minion_id,
                counter: AbilityCounter {
                    id: counter_id,
                    ability: Ability::Disabled,
                    expires_on_effect: Some(EffectQuery::DamageDealt {
                        source: None,
                        target: Some(minion_id.into()),
                    }),
                },
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::TurnStart {
                        player_id: Some(controller_id),
                    },
                    expires_on_effect: None,
                    on_effect: Arc::new(
                        move |state: &State, _card_id: &uuid::Uuid, _effect: &Effect| {
                            let controller_id = controller_id;
                            let minion_id = minion_id;
                            Box::pin(async move {
                                let minion = state.get_card(&minion_id);
                                if !minion.has_ability(state, &Ability::Disabled) {
                                    // Minion is no longer asleep, do nothing.
                                    return Ok(vec![]);
                                }

                                let wake_up = yes_or_no(
                                    &controller_id,
                                    state,
                                    "Dream-Quest: Wake up the dreaming minion?",
                                )
                                .await?;
                                if !wake_up {
                                    return Ok(vec![]);
                                }

                                let mut effects = vec![Effect::RemoveAbility {
                                    card_id: minion_id,
                                    modifier: Ability::Disabled,
                                }];

                                // Draw a spell from the deck.
                                let deck = state.decks.get(&controller_id).unwrap();
                                if let Some(spell_id) = deck.spells.last().cloned() {
                                    effects.push(Effect::SetCardZone {
                                        card_id: spell_id,
                                        zone: Zone::Hand,
                                    });
                                }

                                Ok(effects)
                            })
                                as Pin<
                                    Box<
                                        dyn Future<Output = anyhow::Result<Vec<Effect>>>
                                            + Send
                                            + '_,
                                    >,
                                >
                        },
                    ),
                    multitrigger: false,
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (DreamQuest::NAME, |owner_id: PlayerId| {
    Box::new(DreamQuest::new(owner_id))
});
