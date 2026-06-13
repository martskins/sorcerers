use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct GreatWall {
    site_base: SiteBase,
    card_base: CardBase,
}

impl GreatWall {
    pub const NAME: &'static str = "Great Wall";
    pub const DESCRIPTION: &'static str =
        "Enemy units can’t move through this site’s top border on the ground.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            site_base: SiteBase {
                provided_mana: 1,
                provided_thresholds: Thresholds::ZERO,
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

    fn is_enemy_ground_unit_crossing_top_border(
        &self,
        card_id: &CardId,
        other_side: &Location,
        region: &Region,
        state: &State,
    ) -> bool {
        let card = state.get_card(card_id);
        if !card.is_unit()
            || card.get_controller_id(state) == self.get_controller_id(state)
            || card.has_ability(state, &Ability::Airborne)
            || card.get_region(state) != &Region::Surface
            || region != &Region::Surface
        {
            return false;
        }

        self.get_location()
            .step_in_direction(&Direction::Up, state, Some(card_id))
            .is_some_and(|top_location| &top_location == other_side)
    }
}

#[async_trait::async_trait]
impl Site for GreatWall {
    fn can_be_entered_by(
        &self,
        card_id: &CardId,
        from: &Location,
        region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(
            !self.is_enemy_ground_unit_crossing_top_border(card_id, from, region, state)
                && self.base_can_be_entered_by(card_id, from, region, state)?,
        )
    }

    fn can_be_exited_by(
        &self,
        card_id: &CardId,
        to: &Location,
        region: &Region,
        state: &State,
    ) -> anyhow::Result<bool> {
        Ok(!self.is_enemy_ground_unit_crossing_top_border(card_id, to, region, state))
    }
}

impl ResourceProvider for GreatWall {}

#[async_trait::async_trait]
impl Card for GreatWall {
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
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) = (GreatWall::NAME, |owner_id: PlayerId| {
    Box::new(GreatWall::new(owner_id))
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::ApprenticeWizard;

    #[tokio::test]
    async fn blocks_enemy_ground_movement_through_top_border() {
        let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
        let wall_owner = state.players[0].id;
        let enemy_id = state.players[1].id;

        let mut wall = GreatWall::new(wall_owner);
        wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(wall));

        let mut enemy = ApprenticeWizard::new(enemy_id);
        enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
        state.add_card(Box::new(enemy.clone()));

        let zones = enemy
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(!zones.contains(&Location::Square(8, Region::Surface)));
    }

    #[tokio::test]
    async fn blocks_enemy_ground_movement_out_through_top_border() {
        let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
        let wall_owner = state.players[0].id;
        let enemy_id = state.players[1].id;

        let mut wall = GreatWall::new(wall_owner);
        wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(wall));

        let mut enemy = ApprenticeWizard::new(enemy_id);
        enemy.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(enemy.clone()));

        let zones = enemy
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(!zones.contains(&Location::Square(13, Region::Surface)));
    }

    #[tokio::test]
    async fn allows_allied_and_airborne_movement_through_top_border() {
        let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
        let wall_owner = state.players[0].id;
        let enemy_id = state.players[1].id;

        let mut wall = GreatWall::new(wall_owner);
        wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(wall));

        let mut ally = ApprenticeWizard::new(wall_owner);
        ally.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
        state.add_card(Box::new(ally.clone()));

        let ally_zones = ally
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(ally_zones.contains(&Location::Square(8, Region::Surface)));

        let mut airborne_enemy = ApprenticeWizard::new(enemy_id);
        airborne_enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
        airborne_enemy.add_ability(Ability::Airborne);
        state.add_card(Box::new(airborne_enemy.clone()));

        let airborne_enemy_zones = airborne_enemy
            .get_valid_move_locations(&state)
            .await
            .expect("zones to be computed");
        assert!(airborne_enemy_zones.contains(&Location::Square(8, Region::Surface)));
    }

    #[tokio::test]
    async fn blocks_paths_through_top_border() {
        let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
        let wall_owner = state.players[0].id;
        let enemy_id = state.players[1].id;

        let mut wall = GreatWall::new(wall_owner);
        wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        state.add_card(Box::new(wall));

        let mut enemy = ApprenticeWizard::new(enemy_id);
        enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
        enemy.add_ability(Ability::Movement(1));
        state.add_card(Box::new(enemy.clone()));

        let paths = enemy
            .get_valid_move_paths(&state, &Location::Square(3, Region::Surface))
            .await
            .expect("paths to be computed");
        assert!(paths.is_empty());
    }
}
