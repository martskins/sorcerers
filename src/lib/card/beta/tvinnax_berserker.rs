use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct TvinnaxBerserker {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl TvinnaxBerserker {
    pub const NAME: &'static str = "Tvinnax Berserker";
    pub const DESCRIPTION: &'static str = "When Tvinnax Berserker kills an enemy, untap it.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 3,
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "FF"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for TvinnaxBerserker {
    fn get_name(&self) -> &str { Self::NAME }
    fn get_description(&self) -> &str { Self::DESCRIPTION }
    fn get_base_mut(&mut self) -> &mut CardBase { &mut self.card_base }
    fn get_base(&self) -> &CardBase { &self.card_base }
    fn get_unit_base(&self) -> Option<&UnitBase> { Some(&self.unit_base) }
    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> { Some(&mut self.unit_base) }

    fn on_summon(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let self_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::BuryCard {
                    card: CardQuery::new().units(),
                },
                expires_on_effect: Some(EffectQuery::BuryCard {
                    card: CardQuery::from_id(self_id),
                }),
                on_effect: Arc::new(
                    move |_state: &State, _card_id: &uuid::Uuid, _effect: &Effect| {
                        Box::pin(async move {
                            Ok(vec![Effect::UntapCard { card_id: self_id }])
                        }) as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                    },
                ),
                multitrigger: true,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (TvinnaxBerserker::NAME, |owner_id: PlayerId| {
    Box::new(TvinnaxBerserker::new(owner_id))
});
