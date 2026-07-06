use std::sync::Arc;

use crate::prelude::*;

const NEXT_TURN_HOOK: HookId = 1;

#[derive(Debug, Clone)]
pub struct DreamQuest {
    card_base: CardBase,
    target_id: Option<CardId>,
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
            target_id: None,
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

    fn set_data(
        &mut self,
        data: &std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) -> anyhow::Result<()> {
        if let Some(data) = data.downcast_ref::<CardId>() {
            self.target_id = Some(*data);
        }

        Ok(())
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            NEXT_TURN_HOOK => {
                let Some(target_id) = self.target_id else {
                    return Ok(vec![]);
                };

                let target = state.get_card(&target_id);
                if !target.has_status(state, &CardStatus::Disabled) {
                    // Minion is no longer asleep, do nothing.
                    return Ok(vec![]);
                }

                let controller_id = self.get_controller_id(state);
                let wake_up = yes_or_no(
                    &controller_id,
                    state,
                    "Wake up the dreaming minion?",
                    *self.get_id(),
                )
                .await?;
                if !wake_up {
                    return Ok(vec![]);
                }

                let mut effects = vec![Effect::RemoveStatus {
                    card_id: target_id,
                    status: CardStatus::Disabled,
                }];

                let deck = state.get_player_deck(&controller_id)?.clone();
                if !deck.spells.is_empty() {
                    let Some(chosen) = CardQuery::from_ids(deck.spells.clone())
                        .with_prompt("Choose a card to put into your hand")
                        .with_source_card(*self.get_id())
                        .pick(&controller_id, state)
                        .await?
                    else {
                        return Ok(effects);
                    };
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
                            card_id: target_id,
                            status: CardStatus::Disabled,
                        },
                    ];
                }

                Ok(effects)
            }
            _ => Ok(vec![]),
        }
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
        let spellcaster_abilities = [
            Some(Element::Fire),
            Some(Element::Water),
            Some(Element::Earth),
            Some(Element::Air),
            None,
        ]
        .into_iter()
        .map(Ability::Spellcaster)
        .collect();
        let Some(minion_id) = CardQuery::new()
            .controlled_by(&controller_id)
            .minions()
            .in_play()
            .with_any_ability(spellcaster_abilities)
            .with_prompt("Pick an allied Spellcaster to send on a dream quest")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![
            Effect::AddStatusCounter {
                card_id: minion_id,
                counter: StatusCounter {
                    id: uuid::Uuid::new_v4(),
                    status: CardStatus::Disabled,
                    expires_on_effect: Some(EffectQuery::DamageDealt {
                        source: None,
                        target: Some(minion_id.into()),
                    }),
                },
            },
            Effect::SetCardData {
                card_id: *self.get_id(),
                data: Arc::new(minion_id),
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    hook_id: NEXT_TURN_HOOK,
                    card_id: *self.get_id(),
                    timing: HookTiming::After,
                    trigger_on_effect: EffectQuery::TurnStart {
                        player_id: Some(controller_id),
                    },
                    expires_on_effect: None,
                    trigger_times: Some(1),
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (DreamQuest::NAME, |owner_id: PlayerId| {
    Box::new(DreamQuest::new(owner_id))
});
