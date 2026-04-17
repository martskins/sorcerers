use std::sync::Arc;

use crate::{
    card::{
        Ability, Aura, AuraBase, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region, Zone,
    },
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::{EffectQuery, ZoneQuery},
    state::{CardQuery, ContinuousEffect, DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct EvilPresence {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl EvilPresence {
    pub const NAME: &'static str = "Evil Presence";
    pub const DESCRIPTION: &'static str = "You may summon Spirits to affected sites. When you summon a Spirit here, give it Charge, and return Evil Presence to its owner's hand.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "A"),
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
            aura_base: AuraBase {
                tapped: false,
                region: Region::Surface,
            },
        }
    }
}

impl Aura for EvilPresence {}

#[async_trait::async_trait]
impl Card for EvilPresence {
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

    fn get_aura_base(&self) -> Option<&AuraBase> {
        Some(&self.aura_base)
    }

    fn get_aura_base_mut(&mut self) -> Option<&mut AuraBase> {
        Some(&mut self.aura_base)
    }

    fn get_aura(&self) -> Option<&dyn Aura> {
        Some(self)
    }

    async fn genesis(&self, _state: &State) -> anyhow::Result<Vec<Effect>> {
        let owner_id = *self.get_owner_id();
        let zone = self.get_zone().clone();
        let evil_presence_id = *self.get_id();
        Ok(vec![Effect::AddDeferredEffect {
            effect: DeferredEffect {
                trigger_on_effect: EffectQuery::SummonCard {
                    card: CardQuery::new()
                        .minions()
                        .minion_type(&MinionType::Spirit)
                        .including_not_in_play(),
                },
                expires_on_effect: None,
                on_effect: Arc::new(
                    move |_state: &State, card_id: &uuid::Uuid, _effect: &Effect| {
                        let zone = zone.clone();
                        Box::pin(async move {
                            Ok(vec![
                                Effect::MoveCard {
                                    player_id: owner_id,
                                    card_id: evil_presence_id,
                                    from: zone,
                                    to: ZoneQuery::from_zone(Zone::Hand),
                                    tap: false,
                                    region: Region::Surface,
                                    through_path: None,
                                },
                                Effect::AddAbilityCounter {
                                    card_id: *card_id,
                                    counter: AbilityCounter {
                                        id: uuid::Uuid::new_v4(),
                                        ability: Ability::Charge,
                                        expires_on_effect: None,
                                    },
                                },
                            ])
                        })
                    },
                ),
                multitrigger: false,
            },
        }])
    }

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        Ok(vec![ContinuousEffect::OverrideValidPlayZone {
            affected_zones: self.get_affected_zones(state),
            affected_cards: CardQuery::new()
                .minions()
                .minion_type(&MinionType::Spirit)
                .including_not_in_play(),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (EvilPresence::NAME, |owner_id: PlayerId| {
    Box::new(EvilPresence::new(owner_id))
});
