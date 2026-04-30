use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{
        Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase,
        Zone,
    },
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct MenOfLeng {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl MenOfLeng {
    pub const NAME: &'static str = "Men of Leng";
    pub const DESCRIPTION: &'static str =
        "Whenever Men of Leng strike an Avatar, that Avatar discards a random card.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Beast],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MenOfLeng {
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
        let men_of_leng_id = *self.get_id();

        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::DamageDealt {
                    source: Some(CardQuery::from_id(men_of_leng_id)),
                    target: Some(CardQuery::new().avatars()),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(men_of_leng_id),
                }),
                on_effect: Arc::new(
                    move |state: &State, avatar_id: &uuid::Uuid, _effect: &Effect| {
                        let avatar_id = *avatar_id;
                        Box::pin(async move {
                            let avatar = state.get_card(&avatar_id);
                            let avatar_controller = avatar.get_controller_id(state);

                            let random_card = CardQuery::new()
                                .in_zone(&Zone::Hand)
                                .controlled_by(&avatar_controller)
                                .randomised()
                                .count(1)
                                .pick(&avatar_controller, state, false)
                                .await?;

                            if let Some(card_id) = random_card {
                                Ok(vec![Effect::DiscardCard {
                                    player_id: avatar_controller,
                                    card_id,
                                }])
                            } else {
                                Ok(vec![])
                            }
                        })
                            as Pin<
                                Box<
                                    dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_,
                                >,
                            >
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (MenOfLeng::NAME, |owner_id: PlayerId| {
        Box::new(MenOfLeng::new(owner_id))
    });
