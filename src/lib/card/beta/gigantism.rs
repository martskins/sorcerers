use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Gigantism {
    card_base: CardBase,
}

impl Gigantism {
    pub const NAME: &'static str = "Gigantism";
    pub const DESCRIPTION: &'static str = "Give an allied unit +6 power this turn.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(4, "EE"),
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
impl Card for Gigantism {
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
impl Magic for Gigantism {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let opponent_id = state.get_opponent_id(&controller_id)?;
        let nearby_enemies = CardQuery::new()
            .minions()
            .near_to(self.get_location())
            .controlled_by(&opponent_id)
            .all(state);
        Ok(nearby_enemies
            .into_iter()
            .map(|card_id| Effect::AddCounter {
                card_id,
                counter: Counter {
                    id: uuid::Uuid::new_v4(),
                    power: 6,
                    toughness: 6,
                    expires_on_effect: Some(Box::new(EffectQuery::TurnEnd {
                        player_id: Some(controller_id),
                    })),
                },
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Gigantism::NAME, |owner_id: PlayerId| {
    Box::new(Gigantism::new(owner_id))
});
