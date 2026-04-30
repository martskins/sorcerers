use crate::{
    card::{Card, CardBase, CardConstructor, Cost, Costs, Edition, Rarity, Zone},
    effect::Effect,
    game::PlayerId,
    state::{CardQuery, State},
};

/// **Psionic Blast** — Exceptional Magic (2 cost, A threshold)
///
/// Deal 1 damage to each minion here. They're disabled until your next turn.
#[derive(Debug, Clone)]
pub struct PsionicBlast {
    card_base: CardBase,
}

impl PsionicBlast {
    pub const NAME: &'static str = "Psionic Blast";
    pub const DESCRIPTION: &'static str =
        "Deal 1 damage to each minion here. They're disabled until your next turn.";

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
        }
    }
}

#[async_trait::async_trait]
impl Card for PsionicBlast {
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

    async fn on_cast(
        &mut self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        use crate::{card::Ability, effect::AbilityCounter, query::EffectQuery};

        let controller_id = self.get_controller_id(state);
        let caster_zone = state.get_card(caster_id).get_zone().clone();

        let minions = CardQuery::new().minions().in_zone(&caster_zone).all(state);

        let mut effects: Vec<Effect> = minions
            .iter()
            .map(|&card_id| Effect::TakeDamage {
                card_id,
                from: *caster_id,
                damage: 1,
                is_strike: false,
            })
            .collect();

        for &card_id in &minions {
            effects.push(Effect::AddAbilityCounter {
                card_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Disabled,
                    expires_on_effect: Some(EffectQuery::TurnStart {
                        player_id: Some(controller_id),
                    }),
                },
            });
        }

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (PsionicBlast::NAME, |owner_id: PlayerId| {
    Box::new(PsionicBlast::new(owner_id))
});
