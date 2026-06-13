use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct AstralAlcazar {
    site_base: SiteBase,
    card_base: CardBase,
}

impl AstralAlcazar {
    pub const NAME: &'static str = "Astral Alcazar";
    pub const DESCRIPTION: &'static str =
        "Units can move between this site and any void as if they were adjacent.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 0,
                provided_thresholds: Thresholds::new(),
                types: vec![],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Atlasbook,
                costs: Costs::ZERO,
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }
}

#[async_trait::async_trait]
impl Site for AstralAlcazar {}

impl ResourceProvider for AstralAlcazar {}

#[async_trait::async_trait]
impl Card for AstralAlcazar {
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

    fn get_resource_provider(&self) -> Option<&dyn ResourceProvider> {
        Some(self)
    }

    async fn get_ongoing_effects(&self, state: &State) -> anyhow::Result<Vec<OngoingEffect>> {
        let connected_locations = Location::all_in_region(Region::Void)
            .into_iter()
            .filter(|loc| loc.get_site_at_square(state).is_none())
            .collect();

        Ok(vec![OngoingEffect::ConnectZones {
            connected_locations,
            affected_cards: CardQuery::new().units().in_zone_of_card(self.get_id()),
        }])
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (AstralAlcazar::NAME, |owner_id: PlayerId| {
        Box::new(AstralAlcazar::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::SpectralStalker;

    #[tokio::test]
    async fn connects_site_to_any_void() {
        let mut state = State::new_mock_state(vec![8]);
        let player_id = state.players[0].id;

        let mut alcazar = AstralAlcazar::new(player_id);
        alcazar.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(alcazar));

        let mut unit = SpectralStalker::new(player_id);
        unit.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(unit.clone()));

        state.reconcile_ongoing_effects_for_test().await.unwrap();

        let zones = unit
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(zones.contains(&Location::Square(1, Region::Void)));
        assert!(zones.contains(&Location::Square(20, Region::Void)));
        assert!(!zones.contains(&Location::Square(8, Region::Void)));
    }

    #[tokio::test]
    async fn does_not_connect_any_void_to_site() {
        let mut state = State::new_mock_state(vec![8]);
        let player_id = state.players[0].id;

        let mut alcazar = AstralAlcazar::new(player_id);
        alcazar.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(alcazar));

        let mut unit = SpectralStalker::new(player_id);
        unit.set_zone(Zone::Location(Location::Square(20, Region::Void)));
        state.add_card(Box::new(unit.clone()));

        state.reconcile_ongoing_effects_for_test().await.unwrap();

        let zones = unit
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(zones.contains(&Location::Square(20, Region::Void)));
        assert!(!zones.contains(&Location::Square(1, Region::Void)));
    }
}
