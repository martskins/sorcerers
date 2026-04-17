use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::{EffectQuery, ZoneQuery},
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct CerberusInChains {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl CerberusInChains {
    pub const NAME: &'static str = "Cerberus in Chains";
    pub const DESCRIPTION: &'static str = "Must be summoned to your location.\r \r Cerberus in Chains automatically follows you and can't move itself away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 5,
                toughness: 5,
                abilities: vec![Ability::Immobile],
                types: vec![MinionType::Demon],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
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
impl Card for CerberusInChains {
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

    /// Cerberus must be summoned to the owner's avatar zone.
    fn get_valid_play_zones(&self, state: &State) -> anyhow::Result<Vec<Zone>> {
        let owner_id = self.get_owner_id();
        let avatar_id = state.get_player_avatar_id(owner_id)?;
        let avatar_zone = state.get_card(&avatar_id).get_zone().clone();
        Ok(vec![avatar_zone])
    }

    fn get_valid_move_zones(&self, _state: &State) -> anyhow::Result<Vec<Zone>> {
        Ok(vec![self.get_zone().clone()]) // Cerberus can't move itself.
    }

    fn on_summon(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let cerberus_id = *self.get_id();
        let controller_id = self.get_controller_id(state);

        // DeferredEffect: whenever the controller's avatar moves, Cerberus follows.
        let deferred = DeferredEffect {
            trigger_on_effect: EffectQuery::MoveCard {
                card: CardQuery::new().avatars().controlled_by(&controller_id),
            },
            expires_on_effect: Some(EffectQuery::BuryCard {
                card: self.get_id().into(),
            }),
            on_effect: Arc::new(
                move |state: &State, avatar_card_id: &uuid::Uuid, _effect: &Effect| {
                    let cerberus_id = cerberus_id;
                    let owner_id = controller_id;
                    Box::pin(async move {
                        let cerberus = state.get_card(&cerberus_id);
                        let avatar = state.get_card(avatar_card_id);
                        let new_zone = avatar.get_zone().clone();

                        // Only follow if Cerberus is in play and not already at the same zone.
                        if !cerberus.get_zone().is_in_play() || cerberus.get_zone() == &new_zone {
                            return Ok(vec![]);
                        }

                        let from = cerberus.get_zone().clone();
                        let region = cerberus.get_region(state).clone();
                        Ok(vec![Effect::MoveCard {
                            player_id: owner_id,
                            card_id: cerberus_id,
                            from,
                            to: ZoneQuery::from_zone(new_zone),
                            tap: false,
                            region,
                            through_path: None,
                        }])
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
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (CerberusInChains::NAME, |owner_id: PlayerId| {
        Box::new(CerberusInChains::new(owner_id))
    });
