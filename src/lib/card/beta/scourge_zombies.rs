use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct ScourgeZombies {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl ScourgeZombies {
    pub const NAME: &'static str = "Scourge Zombies";
    pub const DESCRIPTION: &'static str = "When an allied Mortal dies, return Scourge Zombies from cemetery to play.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                types: vec![MinionType::Undead],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "E"),
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
impl Card for ScourgeZombies {
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
                    card: CardQuery::new().minions().minion_type(&MinionType::Mortal),
                },
                expires_on_effect: None,
                on_effect: Arc::new(
                    move |state: &State, buried_id: &uuid::Uuid, _effect: &Effect| {
                        let buried_id = *buried_id;
                        Box::pin(async move {
                            let self_card = state.get_card(&self_id);
                            if *self_card.get_zone() != Zone::Cemetery {
                                return Ok(vec![]);
                            }
                            let self_controller = self_card.get_controller_id(state);
                            let buried_card = state.get_card(&buried_id);
                            if buried_card.get_controller_id(state) != self_controller {
                                return Ok(vec![]);
                            }
                            let valid_zones = self_card.get_valid_play_zones(state, &self_controller)?;
                            let target_zone = match valid_zones.into_iter().next() {
                                Some(z) => z,
                                None => return Ok(vec![]),
                            };
                            Ok(vec![Effect::SummonCard {
                                player_id: self_controller,
                                card_id: self_id,
                                zone: target_zone,
                            }])
                        }) as Pin<Box<dyn Future<Output = anyhow::Result<Vec<Effect>>> + Send + '_>>
                    },
                ),
                multitrigger: false,
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (ScourgeZombies::NAME, |owner_id: PlayerId| {
    Box::new(ScourgeZombies::new(owner_id))
});
