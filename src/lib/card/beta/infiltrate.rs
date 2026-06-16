use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Infiltrate {
    card_base: CardBase,
}

impl Infiltrate {
    pub const NAME: &'static str = "Infiltrate";
    pub const DESCRIPTION: &'static str = "Target enemy minion gains Stealth and taps. You control it until it no longer has Stealth.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "F"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Infiltrate {
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

    fn get_magic(&self) -> Option<&dyn Magic> {
        Some(self)
    }
}

#[async_trait::async_trait]
impl Magic for Infiltrate {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let caster = state.get_card(caster_id);
        let caster_location = caster.get_location().clone();

        let enemy_minions: Vec<CardId> = CardQuery::new()
            .minions()
            .near_to(&caster_location)
            .all(state)
            .into_iter()
            .filter(|id| state.get_card(id).get_controller_id(state) != controller_id)
            .collect();

        if enemy_minions.is_empty() {
            return Ok(vec![]);
        }

        let Some(target_id) = CardQuery::from_ids(enemy_minions)
            .with_prompt("Pick target enemy minion")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };

        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: target_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Stealth,
                    expires_on_effect: None,
                },
            },
            Effect::SetTapped {
                card_id: target_id,
                tapped: true,
            },
            Effect::SetController {
                card_id: target_id,
                player_id: self.get_controller_id(state),
            },
            Effect::AddTemporaryEffect {
                effect: TemporaryEffect::ControllerOverride {
                    controller_id: self.get_controller_id(state),
                    affected_cards: target_id.into(),
                    expires_on_effect: EffectQuery::RemoveAbility {
                        card: target_id.into(),
                        ability: Ability::Stealth,
                    },
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Infiltrate::NAME, |owner_id: PlayerId| {
    Box::new(Infiltrate::new(owner_id))
});
