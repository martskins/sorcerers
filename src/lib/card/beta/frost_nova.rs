use crate::{
    card::{Ability, Card, CardBase, Cost, Costs, Edition, Rarity, Zone},
    effect::{AbilityCounter, Effect},
    game::PlayerId,
    query::EffectQuery,
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct FrostNova {
    pub card_base: CardBase,
}

impl FrostNova {
    pub const NAME: &'static str = "Frost Nova";
    pub const DESCRIPTION: &'static str =
        "Freeze nearby enemy minions. They're disabled until your next turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "WW"),
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
impl Card for FrostNova {
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
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let nearby_enemies = CardQuery::new()
            .minions()
            .near_to(self.get_zone())
            .controlled_by(&opponent_id)
            .all(state);
        Ok(nearby_enemies
            .into_iter()
            .map(|card_id| Effect::AddAbilityCounter {
                card_id,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Disabled,
                    expires_on_effect: Some(EffectQuery::TurnStart {
                        player_id: Some(controller_id),
                    }),
                },
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (FrostNova::NAME, |owner_id: PlayerId| {
        Box::new(FrostNova::new(owner_id))
    });
