use crate::{
    card::{
        Ability, Aura, AuraBase, Card, CardBase, CardBaseMethods, CardConstructor, Costs, Edition,
        Rarity, Region, Zone,
    },
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, ContinuousEffect, State},
};

#[derive(Debug, Clone)]
pub struct Sandstorm {
    aura_base: AuraBase,
    card_base: CardBase,
}

impl Sandstorm {
    pub const NAME: &'static str = "Sandstorm";
    pub const DESCRIPTION: &'static str =
        "Affected sites and units atop them can't be attacked or intercepted.

At the start of your turn, dispel Sandstorm.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(1, "F"),
                rarity: Rarity::Ordinary,
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

impl Aura for Sandstorm {
    fn should_dispell(&self, state: &State) -> anyhow::Result<bool> {
        let controller_id = self.get_controller_id(state);
        let turns_in_play = state
            .effect_log
            .iter()
            .skip_while(|e| {
                !matches!(*e.effect, Effect::PlayCard { ref card_id, .. } if card_id == self.get_id())
            })
            .filter(|e| {
                matches!(*e.effect, Effect::StartTurn { ref player_id, .. } if player_id == &controller_id)
            })
            .count();

        Ok(turns_in_play >= 1)
    }

    fn get_affected_zones(&self, state: &State) -> Vec<Zone> {
        self.base_get_affected_zones(state)
            .into_iter()
            .filter(|zone| zone.get_site(state).is_some())
            .collect()
    }
}

#[async_trait::async_trait]
impl Card for Sandstorm {
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

    async fn get_continuous_effects(&self, state: &State) -> anyhow::Result<Vec<ContinuousEffect>> {
        let affected_zones = self.get_affected_zones(state);
        if affected_zones.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![
            ContinuousEffect::GrantAbility {
                ability: Ability::Unattackable,
                affected_cards: CardQuery::new().sites().in_zones(&affected_zones),
            },
            ContinuousEffect::GrantAbility {
                ability: Ability::Unattackable,
                affected_cards: CardQuery::new().units().in_zones(&affected_zones),
            },
            ContinuousEffect::GrantAbility {
                ability: Ability::Uninterceptable,
                affected_cards: CardQuery::new().units().in_zones(&affected_zones),
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Sandstorm::NAME, |owner_id: PlayerId| {
    Box::new(Sandstorm::new(owner_id))
});
