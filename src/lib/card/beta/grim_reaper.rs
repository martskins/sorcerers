use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct GrimReaper {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl GrimReaper {
    pub const NAME: &'static str = "Grim Reaper";
    pub const DESCRIPTION: &'static str = "Lethal\r \r Whenever Grim Reaper kills a minion, banish that minion and all copies. Search its owner's cemetery, hand, and spellbook and banish any copies. They shuffle.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Lethal],
                types: vec![MinionType::Spirit],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "AA"),
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
impl Card for GrimReaper {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let grim_reaper_id = *self.get_id();

        let deferred = DeferredEffect {
            trigger_on_effect: EffectQuery::BuryCard {
                card: CardQuery::new().minions(),
            },
            expires_on_effect: Some(EffectQuery::BuryCard {
                card: self.get_id().into(),
            }),
            on_effect: Arc::new(
                move |state: &State, buried_card_id: &uuid::Uuid, _effect: &Effect| {
                    let buried_card_id = *buried_card_id;
                    Box::pin(async move {
                        // Check if Grim Reaper was the killer.
                        let was_killed_by_reaper = state.effect_log.iter().rev().any(|le| {
                            matches!(*le.effect, Effect::KillMinion { ref card_id, ref killer_id }
                                if card_id == &buried_card_id && killer_id == &grim_reaper_id)
                        });

                        if !was_killed_by_reaper {
                            return Ok(vec![]);
                        }

                        // Get the name of the buried card to banish all copies.
                        let buried_name = state.get_card(&buried_card_id).get_name().to_string();
                        let buried_owner_id =
                            state.get_card(&buried_card_id).get_controller_id(state);

                        // Banish the killed minion.
                        let mut effects = vec![Effect::BanishCard {
                            card_id: buried_card_id,
                        }];

                        // Find all copies of that card (by name) in any zone belonging to its owner.
                        let copies: Vec<uuid::Uuid> = state
                            .cards
                            .iter()
                            .filter(|c| c.get_name().eq_ignore_ascii_case(&buried_name))
                            .filter(|c| c.get_id() != &buried_card_id)
                            .filter(|c| c.get_controller_id(state) == buried_owner_id)
                            .map(|c| *c.get_id())
                            .collect();

                        for copy_id in copies {
                            effects.push(Effect::BanishCard { card_id: copy_id });
                        }

                        effects.push(Effect::ShuffleDeck {
                            player_id: buried_owner_id,
                        });

                        Ok(effects)
                    })
                        as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                },
            ),
            multitrigger: true,
        };

        Ok(vec![Effect::AddDeferredEffect { effect: deferred }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GrimReaper::NAME, |owner_id: PlayerId| {
    Box::new(GrimReaper::new(owner_id))
});
