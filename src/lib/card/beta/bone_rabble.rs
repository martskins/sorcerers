use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::{Element, PlayerId, yes_or_no},
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct BoneRabble {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl BoneRabble {
    pub const NAME: &'static str = "Bone Rabble";
    pub const DESCRIPTION: &'static str =
        "Whenever you play an earth site, you may summon Bone Rabble from your cemetery to that site.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 1,
                toughness: 1,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Beast],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "E"),
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
impl Card for BoneRabble {
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

    fn deathrite(&self, _state: &State, _from: &Zone) -> Vec<Effect> {
        let owner_id = self.get_owner_id().clone();
        let bone_rabble_id = self.get_id().clone();
        vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::PlayCard {
                    card: CardQuery::new()
                        .with_element(Element::Earth)
                        .controlled_by(&owner_id)
                        .sites(),
                },
                expires_on_effect: Some(EffectQuery::SummonCard {
                    card: CardQuery::from_id(self.get_id().clone()),
                }),
                on_effect: Arc::new(move |state: &State, card_id: &uuid::Uuid, _effect: &Effect| {
                    let owner_id = owner_id.clone();
                    Box::pin(async move {
                        let site = state.get_card(card_id);
                        let summon_bone_rabble =
                            yes_or_no(&owner_id, state, "Summon Bone Rabble atop the played site?").await?;
                        if summon_bone_rabble {
                            Ok(vec![Effect::SummonCard {
                                player_id: owner_id.clone(),
                                card_id: bone_rabble_id.clone(),
                                zone: site.get_zone().clone(),
                            }])
                        } else {
                            Ok(vec![])
                        }
                    }) as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                }),
            },
        }]
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BoneRabble::NAME, |owner_id: PlayerId| {
    Box::new(BoneRabble::new(owner_id))
});
