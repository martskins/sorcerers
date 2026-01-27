use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, pick_card},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Blaze {
    pub card_base: CardBase,
}

impl Blaze {
    pub const NAME: &'static str = "Blaze";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(3, "F"),
                region: Region::Surface,
                rarity: Rarity::Exceptional,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
                is_token: false,
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Blaze {
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
        let units = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_controller_id(state) == self.get_controller_id(state))
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let prompt = "Blaze: Pick an ally";
        let picked_card = pick_card(self.get_controller_id(state), &units, state, prompt).await?;
        Ok(vec![
            Effect::AddAbilityCounter {
                card_id: picked_card.clone(),
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Movement(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                },
            },
            Effect::AddAbilityCounter {
                card_id: picked_card,
                counter: AbilityCounter {
                    id: uuid::Uuid::new_v4(),
                    ability: Ability::Blaze(2),
                    expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
                },
            },
        ])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Blaze::NAME, |owner_id: PlayerId| Box::new(Blaze::new(owner_id)));
