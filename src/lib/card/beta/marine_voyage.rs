use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct MarineVoyage {
    card_base: CardBase,
}

impl MarineVoyage {
    pub const NAME: &'static str = "Marine Voyage";
    pub const DESCRIPTION: &'static str = "This turn, your units can move between any sites in a chosen body of water as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(2, "W"),
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
impl Card for MarineVoyage {
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
impl Magic for MarineVoyage {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let bodies_of_water = state
            .get_bodies_of_water()
            .into_iter()
            .map(|body| body.iter().map(Zone::from).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let prompt = "Pick a body of water";
        let body_of_water = pick_zone_group_source(
            controller_id,
            &bodies_of_water,
            state,
            false,
            prompt,
            Some(*self.get_id()),
        )
        .await?;

        Ok(vec![Effect::AddTemporaryEffect {
            effect: TemporaryEffect::ConnectSites {
                sites: body_of_water,
                affected_cards: CardQuery::new().units().controlled_by(&controller_id),
                expires_on_effect: EffectQuery::TurnEnd {
                    player_id: Some(controller_id),
                },
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (MarineVoyage::NAME, |owner_id: PlayerId| {
    Box::new(MarineVoyage::new(owner_id))
});
