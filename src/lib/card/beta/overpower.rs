use crate::{
    card::{Card, CardBase, Edition, Plane, Rarity, Zone},
    effect::{Counter, Effect},
    game::{PlayerId, Thresholds, pick_card},
    query::EffectQuery,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Overpower {
    pub card_base: CardBase,
}

impl Overpower {
    pub const NAME: &'static str = "Overpower";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                tapped: false,
                zone: Zone::Spellbook,
                mana_cost: 1,
                required_thresholds: Thresholds::parse("E"),
                plane: Plane::Surface,
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Overpower {
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
        let allies = state
            .cards
            .iter()
            .filter(|c| c.get_zone().is_in_play())
            .filter(|c| c.get_controller_id() == self.get_controller_id())
            .filter(|c| c.is_unit())
            .map(|c| c.get_id())
            .cloned()
            .collect::<Vec<_>>();
        let picked_ally_id = pick_card(
            self.get_controller_id(),
            &allies,
            state,
            "Overpower: Pick an ally to give +2 power to",
        )
        .await?;

        Ok(vec![Effect::AddCounter {
            card_id: picked_ally_id,
            counter: Counter {
                id: uuid::Uuid::new_v4(),
                power: 2,
                toughness: 2,
                expires_on_effect: Some(EffectQuery::TurnEnd { player_id: None }),
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, fn(PlayerId) -> Box<dyn Card>) =
    (Overpower::NAME, |owner_id: PlayerId| Box::new(Overpower::new(owner_id)));