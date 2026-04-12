use std::sync::Arc;

use crate::{
    card::{Card, CardBase, Costs, Edition, MinionType, Rarity, Region, UnitBase, Zone},
    effect::Effect,
    game::PlayerId,
    query::EffectQuery,
    state::{DeferredEffect, State},
};

#[derive(Debug, Clone)]
pub struct BridgeTroll {
    pub unit_base: UnitBase,
    pub card_base: CardBase,
}

impl BridgeTroll {
    pub const NAME: &'static str = "Bridge Troll";
    pub const DESCRIPTION: &'static str = "Whenever an enemy attacks Bridge Troll, they must spend all of their remaining mana to give to you on your next turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 3,
                toughness: 4,
                abilities: vec![],
                types: vec![MinionType::Mortal],
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "W"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for BridgeTroll {
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

    fn on_defend(&self, state: &State, attacker_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let attacker = state.get_card(attacker_id);
        let attacker_controller = attacker.get_controller_id(state);
        let my_controller = self.get_controller_id(state);

        let enemy_mana = *state.player_mana.get(&attacker_controller).unwrap_or(&0);
        let mut effects = vec![
            Effect::ConsumeMana {
                player_id: attacker_controller,
                mana: enemy_mana,
            },
            Effect::AddDeferredEffect {
                effect: DeferredEffect {
                    trigger_on_effect: EffectQuery::TurnStart {
                        player_id: Some(my_controller.clone()),
                    },
                    expires_on_effect: None,
                    on_effect: Arc::new(move |_: &State, _: &uuid::Uuid, _: &Effect| {
                        Box::pin(async move {
                            Ok(vec![Effect::AddMana {
                                player_id: my_controller.clone(),
                                mana: enemy_mana,
                            }])
                        })
                    }),
                    multitrigger: false,
                },
            },
        ];

        // Strike back as normal defender.
        if let Some(power) = self.get_power(state)? {
            effects.push(Effect::TakeDamage {
                card_id: attacker_id.clone(),
                from: self.get_id().clone(),
                damage: power,
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) = (BridgeTroll::NAME, |owner_id: PlayerId| {
    Box::new(BridgeTroll::new(owner_id))
});
