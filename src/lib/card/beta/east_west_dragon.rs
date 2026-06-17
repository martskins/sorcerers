use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct EastWestDragon {
    unit_base: UnitBase,
    card_base: CardBase,
}

impl EastWestDragon {
    pub const NAME: &'static str = "East-West Dragon";
    pub const DESCRIPTION: &'static str = "Airborne\r Moves freely sideways.";

    pub fn new(owner_id: PlayerId) -> Self {
        Self {
            unit_base: UnitBase {
                power: 4,
                toughness: 4,
                abilities: vec![Ability::Airborne],
                types: vec![MinionType::Dragon],
                tapped: false,
                ..Default::default()
            },
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                zone: Zone::Spellbook,
                costs: Costs::basic(5, "AA"),
                rarity: Rarity::Elite,
                edition: Edition::Beta,
                controller_id: owner_id,
                is_token: false,
                ..Default::default()
            },
        }
    }

    fn is_sideways_move(from: &Location, to: &Location) -> bool {
        let (Some(from_square), Some(to_square)) = (from.square(), to.square()) else {
            return false;
        };

        from.region() == to.region()
            && from_square != to_square
            && (from_square - 1) / 5 == (to_square - 1) / 5
    }

    fn sideways_locations(&self, state: &State) -> Vec<Location> {
        let Location::Square(square, region) = self.get_location() else {
            return vec![];
        };

        let row_start = ((square - 1) / 5) * 5 + 1;
        let row_end = row_start + 4;
        let from = self.get_location().clone();

        (row_start..=row_end)
            .map(|square| Location::Square(square, region.clone()))
            .filter(|location| location != &from)
            .filter(|location| location.is_location(state))
            .filter(|location| {
                self.can_move_between_locations(state, &from, location)
                    .unwrap_or(false)
            })
            .collect()
    }

    fn is_valid_sideways_destination(&self, state: &State, to: &Location) -> bool {
        Self::is_sideways_move(self.get_location(), to)
            && to.is_location(state)
            && self
                .can_move_between_locations(state, self.get_location(), to)
                .unwrap_or(false)
    }
}

#[async_trait::async_trait]
impl Card for EastWestDragon {
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

    fn get_unit_base(&self) -> Option<&UnitBase> {
        Some(&self.unit_base)
    }

    fn get_unit_base_mut(&mut self) -> Option<&mut UnitBase> {
        Some(&mut self.unit_base)
    }

    async fn get_valid_move_locations(&self, state: &State) -> anyhow::Result<Vec<Location>> {
        let mut locations = self.base_valid_move_locations(state).await?;
        locations.extend(self.sideways_locations(state));
        locations.sort();
        locations.dedup();
        Ok(locations)
    }

    async fn get_valid_move_paths(
        &self,
        state: &State,
        to: &Location,
    ) -> anyhow::Result<Vec<Vec<Location>>> {
        let from = self.get_location().clone();
        let valid_locations = self.get_valid_move_locations(state).await?;
        if !valid_locations.contains(to) {
            return Ok(vec![]);
        }

        let mut paths = vec![];
        if self.is_valid_sideways_destination(state, to) {
            paths.push(vec![from.clone(), to.clone()]);
        }

        let max_steps = self.get_steps_per_movement(state)?;
        let is_traversable = |current: &Location, next: &Location| -> anyhow::Result<bool> {
            Ok(self
                .get_locations_within_steps_of(state, 1, current)
                .contains(next)
                && self.can_move_between_locations(state, current, next)?)
        };

        let mut queue: Vec<(Vec<Location>, Location)> = vec![(vec![from.clone()], from)];
        while let Some((path, current)) = queue.pop() {
            if &current == to {
                if path.len() - 1 <= max_steps.into() && !paths.contains(&path) {
                    paths.push(path.clone());
                }
                continue;
            }
            if path.len() > max_steps.into() {
                continue;
            }
            for next in valid_locations.iter() {
                if path.contains(next) || &current == next {
                    continue;
                }
                if is_traversable(&current, next)? {
                    let mut new_path = path.clone();
                    new_path.push(next.clone());
                    queue.push((new_path, next.clone()));
                }
            }
        }

        Ok(paths)
    }
}

#[linkme::distributed_slice(crate::card::ALL_CARDS)]
static CONSTRUCTOR: (&'static str, CardConstructor) =
    (EastWestDragon::NAME, |owner_id: PlayerId| {
        Box::new(EastWestDragon::new(owner_id))
    });

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn east_west_dragon_keeps_normal_airborne_movement() {
        let mut state = State::new_mock_state(vec![7, 8, 9, 12, 13, 14]);
        let player_id = state.players[0].id;
        let mut dragon = EastWestDragon::new(player_id);
        dragon.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        let dragon_id = *dragon.get_id();
        state.add_card(Box::new(dragon));

        let locations = state
            .get_card(&dragon_id)
            .get_valid_move_locations(&state)
            .await
            .expect("move locations to be computed");

        assert!(
            locations.contains(&Location::Square(14, Region::Surface)),
            "East-West Dragon should keep normal Airborne movement; got {:?}",
            locations
        );
    }

    #[tokio::test]
    async fn east_west_dragon_moves_sideways_freely_with_direct_path() {
        let mut state = State::new_mock_state(vec![6, 7, 8, 9, 10]);
        let player_id = state.players[0].id;
        let mut dragon = EastWestDragon::new(player_id);
        dragon.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
        let dragon_id = *dragon.get_id();
        state.add_card(Box::new(dragon));

        let target = Location::Square(10, Region::Surface);
        let card = state.get_card(&dragon_id);
        let locations = card
            .get_valid_move_locations(&state)
            .await
            .expect("move locations to be computed");
        let paths = card
            .get_valid_move_paths(&state, &target)
            .await
            .expect("move paths to be computed");

        assert!(
            locations.contains(&target),
            "East-West Dragon should be able to move freely sideways; got {:?}",
            locations
        );
        assert!(
            paths.contains(&vec![Location::Square(8, Region::Surface), target]),
            "East-West Dragon should have a direct free-sideways path; got {:?}",
            paths
        );
    }
}
