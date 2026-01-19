use crate::{
    card::{Ability, Card, CardBase, Cost, Edition, Rarity, Region, Zone},
    effect::{AbilityCounter, Effect},
    game::{PlayerId, pick_card},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct MadDash {
    pub card_base: CardBase,
}

impl MadDash {
    pub const NAME: &'static str = "Mad Dash";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                cost: Cost::new(2, "F"),
                region: Region::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for MadDash {
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
        let mut effects = vec![Effect::DrawCard {
            player_id: self.get_controller_id(state).clone(),
            count: 1,
        }];
        let cards = state
            .cards
            .iter()
            .filter(|c| c.is_unit())
            .filter(|c| c.get_controller_id(state) == self.get_controller_id(state))
            .map(|c| c.get_id().clone())
            .collect::<Vec<uuid::Uuid>>();
        let prompt = "Mad Dash: Pick a unit to gain Movement +1";
        let picked_card_id = pick_card(self.get_controller_id(state), &cards, state, prompt).await?;
        effects.push(Effect::AddAbilityCounter {
            card_id: picked_card_id.clone(),
            counter: AbilityCounter {
                id: uuid::Uuid::new_v4(),
                ability: Ability::Movement(1),
                expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
            },
        });

        Ok(effects)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (MadDash::NAME, |owner_id: PlayerId| Box::new(MadDash::new(owner_id)));
