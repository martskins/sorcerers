use crate::{
    card::{Card, CardBase, CardConstructor, Costs, Edition, Rarity, ResourceProvider, Site, SiteBase, Zone},
    effect::Effect,
    game::{PlayerId, Thresholds, pick_zone},
    state::{CardQuery, State},
};

#[derive(Debug, Clone)]
pub struct Maelström {
    site_base: SiteBase,
    card_base: CardBase,
}

impl Maelström {
    pub const NAME: &'static str = "Maelström";
    pub const DESCRIPTION: &'static str =
        "At the start of your turn, you may pull in each minion in this body of water one step.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::parse("W"),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Unique,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

impl Site for Maelström {}

#[async_trait::async_trait]
impl Card for Maelström {
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

    fn get_site_base(&self) -> Option<&SiteBase> {
        Some(&self.site_base)
    }

    fn get_site_base_mut(&mut self) -> Option<&mut SiteBase> {
        Some(&mut self.site_base)
    }

    fn get_site(&self) -> Option<&dyn Site> {
        Some(self)
    }

    async fn on_turn_start(&self, state: &State) -> anyhow::Result<Vec<Effect>> {
        let controller_id = self.get_controller_id(state);
        if state.current_player != controller_id {
            return Ok(vec![]);
        }
        let body_of_water = state
            .get_body_of_water_at(self.get_zone())
            .unwrap_or_default();
        let minion_ids = CardQuery::new()
            .minions()
            .in_zones(&body_of_water)
            .all(state);

        let mut effects = vec![];
        for minion_id in minion_ids {
            let minion = state.get_card(&minion_id);
            let steps = minion
                .get_zone()
                .steps_to_zone(self.get_zone())
                .unwrap_or_default();
            let mut zones = minion.get_zones_within_steps(state, steps);
            zones.retain(|zone| body_of_water.contains(zone));
            zones.retain(|zone| zone.steps_to_zone(self.get_zone()).unwrap_or_default() <= steps);

            let prompt = format!(
                "Maelström: Pick a zone to move {}({}) to, or pick its current zone to not move it",
                minion.get_name(),
                minion.get_zone().get_square().unwrap_or_default()
            );
            let picked_zone = pick_zone(controller_id, &zones, state, true, &prompt).await?;
            if &picked_zone != minion.get_zone() {
                effects.push(Effect::MoveCard {
                    card_id: minion_id,
                    player_id: controller_id,
                    from: minion.get_zone().clone(),
                    to: picked_zone.into(),
                    tap: false,
                    region: minion.get_region(state).clone(),
                    through_path: None,
                });
            }
        }

        Ok(effects)
    }

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (Maelström::NAME, |owner_id: PlayerId| {
    Box::new(Maelström::new(owner_id))
});
