use std::{future::Future, pin::Pin, sync::Arc};

use crate::prelude::*;

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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for DreamQuest {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let dream_quest_id = *self.get_id();

        // Find all allied Spellcaster minions in play.
        let all_minions = CardQuery::new()
            .controlled_by(&controller_id)
            .minions()
            .in_play()
            .all(state);
        let spellcasters: Vec<CardId> = all_minions
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
            .with_prompt("Pick an allied Spellcaster to send on a dream quest")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?;

        let Some(minion_id) = picked_id else {
            return Ok(vec![]);
        };

        let counter_id = uuid::Uuid::new_v4();

        Ok(vec![
            Effect::AddStatusCounter {
                card_id: minion_id,
                counter: StatusCounter {
                    id: counter_id,
                    status: CardStatus::Disabled,
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
                        move |state: &State, _card_id: &CardId, _effect: &Effect| {
                            let controller_id = controller_id;
                            let minion_id = minion_id;
                            Box::pin(async move {
                                let minion = state.get_card(&minion_id);
                                if !minion.has_status(state, &CardStatus::Disabled) {
                                    // Minion is no longer asleep, do nothing.
                                    return Ok(vec![]);
                                }

                                let wake_up = yes_or_no_source(
                                    &controller_id,
                                    state,
                                    "Wake up the dreaming minion?",
                                    Some(dream_quest_id),
                                )
                                .await?;
                                if !wake_up {
                                    return Ok(vec![]);
                                }

                                let mut effects = vec![Effect::RemoveStatus {
                                    card_id: minion_id,
                                    status: CardStatus::Disabled,
                                }];

                                let deck = state.get_player_deck(&controller_id)?.clone();
                                if !deck.spells.is_empty() {
                                    let chosen = pick_card_with_options(
                                        &controller_id,
                                        &deck.spells,
                                        &deck.spells,
                                        false,
                                        state,
                                        "Choose a card to put into your hand",
                                    )
                                    .await?;
                                    let mut spells = deck.spells.clone();
                                    spells.retain(|id| id != &chosen);
                                    effects = vec![
                                        Effect::ShuffleDeck {
                                            player_id: controller_id,
                                        },
                                        Effect::RearrangeDeck {
                                            spells,
                                            sites: deck.sites.clone(),
                                        },
                                        Effect::SetCardZone {
                                            card_id: chosen,
                                            zone: Zone::Hand,
                                        },
                                        Effect::RemoveStatus {
                                            card_id: minion_id,
                                            status: CardStatus::Disabled,
                                        },
                                    ];
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
