use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, Thresholds},
    query::EffectQuery,
    state::{CardMatcher, State},
};

#[derive(Debug, Clone)]
pub struct FrostNova {
    pub card_base: CardBase,
}

impl FrostNova {
    pub const NAME: &'static str = "Frost Nova";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(4, "WW"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for FrostNova {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    async fn on_cast(&mut self, state: &State, _caster_id: &uuid::Uuid) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let nearby_enemies = CardMatcher::minions_near(self.get_zone())
            .controller_id(&controller_id)
            .resolve_ids(state);
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
    (FrostNova::NAME, |owner_id: PlayerId| Box::new(FrostNova::new(owner_id)));