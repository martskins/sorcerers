use std::sync::Arc;

use crate::{
    card::{
        Ability, Card, CardBase, CardConstructor, Costs, Edition, MinionType, Rarity, Region,
        UnitBase, Zone,
    },
    effect::Effect,
    game::{PlayerId, reveal_cards},
    query::EffectQuery,
    state::{CardQuery, ContinuousEffect, State},
};

async fn reveal_enemy_hands_in_range(
    scout_id: uuid::Uuid,
    state: &State,
) -> anyhow::Result<Vec<Effect>> {
    let scout = state.get_card(&scout_id);
    if !scout.get_zone().is_in_play() {
        return Ok(vec![]);
    }

    let controller_id = scout.get_controller_id(state);
    let range = scout.get_steps_per_movement(state).unwrap_or(0);
    let zones = scout.get_zones_within_steps(state, range);

    for avatar_id in CardQuery::from_ids(state.cards.iter().map(|card| *card.get_id()).collect())
        .in_zones(&zones)
        .all(state)
        .into_iter()
        .filter(|card_id| {
            let card = state.get_card(card_id);
            card.is_avatar() && card.get_controller_id(state) != controller_id
        })
    {
        let avatar = state.get_card(&avatar_id);
        let hand: Vec<uuid::Uuid> = state
            .cards
            .iter()
            .filter(|card| card.get_zone() == &Zone::Hand)
            .filter(|card| card.get_owner_id() == avatar.get_owner_id())
            .map(|card| *card.get_id())
            .collect();
        if hand.is_empty() {
            continue;
        }

        let player = state.get_player(avatar.get_owner_id())?;
        reveal_cards(
            &controller_id,
            &hand,
            state,
            &format!("Swiven Scout: Seeing {}'s hand", player.name),
        )
        .await?;
    }

    Ok(vec![])
}

#[derive(Debug, Clone)]
pub struct SwivenScout {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl SwivenScout {
    pub const NAME: &'static str = "Swiven Scout";
    pub const DESCRIPTION: &'static str = "Movement +1 Enemy Avatars within Swiven Scout's range of motion play with their hands revealed.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 2,
                toughness: 2,
                abilities: vec![Ability::Movement(1)],
                types: vec![MinionType::Mortal],
                tapped: false,
                region: Region::Surface,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "F"),
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
impl Card for SwivenScout {
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

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        reveal_enemy_hands_in_range(*self.get_id(), state).await
    }

    async fn get_continuous_effects(
        &self,
        _state: &State,
    ) -> anyhow::Result<Vec<ContinuousEffect>> {
        let scout_id = *self.get_id();
        Ok(vec![ContinuousEffect::TriggeredEffect {
            trigger_on_effect: EffectQuery::MoveCard {
                card: CardQuery::new().units(),
            },
            on_effect: Arc::new(
                move |state: &State, _card_id: &uuid::Uuid, _effect: &Effect| {
                    Box::pin(async move { reveal_enemy_hands_in_range(scout_id, state).await })
                },
            ),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (SwivenScout::NAME, |owner_id: PlayerId| {
    Box::new(SwivenScout::new(owner_id))
});
