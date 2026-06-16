use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Boil {
    card_base: CardBase,
}

impl Boil {
    pub const NAME: &'static str = "Boil";
    pub const DESCRIPTION: &'static str =
        "Destroy all minions occupying target water site up to two steps away.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(3, "FF"),
                rarity: Rarity::Ordinary,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Card for Boil {
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
impl Magic for Boil {
    async fn resolve_magic(
        &self,
        state: &State,
        caster_id: &uuid::Uuid,
        _cost_paid: Cost,
    ) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        let card = state.get_card(caster_id);
        let zones = card.get_locations_within_steps(state, 2);
        let Some(picked_site_id) = CardQuery::new()
            .water_sites()
            .in_locations(&zones)
            .pick(&controller_id, state)
            .await?
        else {
            return Ok(vec![]);
        };
        let site = state.get_card(&picked_site_id);
        Ok(CardQuery::new()
            .minions()
            .occupying_site_at_location(site.get_location().clone())
            .all(state)
            .into_iter()
            .map(|minion_id| Effect::KillMinion {
                card_id: minion_id,
                killer_id: *caster_id,
                from_attack: false,
            })
            .collect())
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Boil::NAME, |owner_id: PlayerId| {
    Box::new(Boil::new(owner_id))
});
