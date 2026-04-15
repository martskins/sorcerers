use std::sync::Arc;

use crate::{
    card::{Ability, Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::{EffectQuery, ZoneQuery},
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct AwakenedMummies {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl AwakenedMummies {
    pub const NAME: &'static str = "Awakened Mummies";
    pub const DESCRIPTION: &'static str = "Summon Awakened Mummies burrowed safely. When an enemy unit moves onto the ground above them, they unburrow and fight.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                abilities: vec![Ability::Burrowing],
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "F"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn burrow_trigger(&self, state: &State) -> anyhow::Result<DeferredEffect> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let mummy_id = self.get_id().clone();
        let zone = self.get_zone().clone();

        Ok(DeferredEffect {
            trigger_on_effect: EffectQuery::EnterZone {
                card: CardQuery::new()
                    .units()
                    .in_region(&Region::Surface)
                    .controlled_by(&opponent_id),
                zone: ZoneQuery::from_zone(zone),
            },
            expires_on_effect: Some(EffectQuery::BuryCard {
                card: self.get_id().into(),
            }),
            on_effect: Arc::new(
                move |state: &State, card_id: &uuid::Uuid, _effect: &Effect| {
                    let mummy_id = mummy_id.clone();
                    Box::pin(async move {
                        let mummy = state.get_card(&mummy_id);
                        if mummy.get_region(state) != &Region::Underground {
                            return Ok(vec![]);
                        }

                        Ok(vec![
                            Effect::SetCardRegion {
                                card_id: mummy_id.clone(),
                                region: Region::Surface,
                                tap: false,
                            },
                            Effect::Attack {
                                attacker_id: mummy_id.clone(),
                                defender_id: card_id.clone(),
                            },
                        ])
                    })
                },
            ),
            multitrigger: true,
        })
    }
}

#[async_trait::async_trait]
impl Card for AwakenedMummies {
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

    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        Ok(vec![
            Effect::SetCardRegion {
                card_id: self.get_id().clone(),
                region: Region::Underground,
                tap: false,
            },
            Effect::AddDeferredEffect {
                effect: self.burrow_trigger(state)?,
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (AwakenedMummies::NAME, |owner_id: PlayerId| {
        Box::new(AwakenedMummies::new(owner_id))
    });
