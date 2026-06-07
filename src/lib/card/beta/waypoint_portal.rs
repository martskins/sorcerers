use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct WaypointPortal {
    card_base: CardBase,
}

impl WaypointPortal {
    pub const NAME: &'static str = "Waypoint Portal";
    pub const DESCRIPTION: &'static str = "Choose two different sites. This turn, units can move between them as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "A"),
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
impl Card for WaypointPortal {
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
impl Magic for WaypointPortal {
    async fn resolve_magic(
        &self,
        state: &State,
        _caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let Some(first_site_id) = CardQuery::new()
            .sites()
            .in_play()
            .with_prompt("Pick the first site")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let Some(second_site_id) = CardQuery::new()
            .sites()
            .in_play()
            .id_not(first_site_id)
            .with_prompt("Pick the second site")
            .with_source_card(*self.get_id())
            .pick(&controller_id, state, false)
            .await?
        else {
            return Ok(vec![]);
        };
        let first_zone = state.get_card(&first_site_id).get_zone().clone();
        let second_zone = state.get_card(&second_site_id).get_zone().clone();

        Ok(vec![Effect::AddTemporaryEffect {
            effect: TemporaryEffect::ConnectSites {
                sites: vec![first_zone, second_zone],
                affected_cards: CardQuery::new().units(),
                expires_on_effect: EffectQuery::TurnEnd { player_id: None },
            },
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (WaypointPortal::NAME, |owner_id: PlayerId| {
        Box::new(WaypointPortal::new(owner_id))
    });
