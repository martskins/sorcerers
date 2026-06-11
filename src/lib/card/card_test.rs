use crate::{
    card::{
        Ability, AdditionalCost, ApprenticeWizard, AridDesert, AstralAlcazar, AwakenedMummies,
        Card, Cost, Drought, GreatWall, OgreGoons, Region, RimlandNomads, RootSpider,
        SimpleVillage, SpectralStalker, SpringRiver,
    },
    query::CardQuery,
    state::State,
    zone::{Location, Zone},
};

#[test]
fn test_additional_cost_tap() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let cost = Cost::additional_only(AdditionalCost::tap(
        CardQuery::new()
            .untapped()
            .units()
            .in_location(Location::Square(10, Region::Surface)),
    ));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "no units in the zone");

    let mut unit = ApprenticeWizard::new(player_id);
    let unit_id = *unit.get_id();
    unit.set_zone(Zone::Location(Location::Square(10, Region::Surface)));
    state.cards.insert(*unit.get_id(), Box::new(unit));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(can_afford, "an untapped unit is present in the zone");

    let unit = state.get_card_mut(&unit_id);
    unit.set_tapped(true);
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "only unit in zone is tapped");
}

#[tokio::test]
async fn test_astral_alcazar_connects_site_to_any_void() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;

    let mut alcazar = AstralAlcazar::new(player_id);
    alcazar.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*alcazar.get_id(), Box::new(alcazar));

    let mut unit = SpectralStalker::new(player_id);
    unit.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*unit.get_id(), Box::new(unit.clone()));

    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let zones = unit
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(zones.contains(&Zone::Location(Location::Square(1, Region::Void))));
    assert!(zones.contains(&Zone::Location(Location::Square(20, Region::Void))));
    assert!(!zones.contains(&Zone::Location(Location::Square(8, Region::Void))));
}

#[tokio::test]
async fn test_astral_alcazar_does_not_connect_any_void_to_site() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;

    let mut alcazar = AstralAlcazar::new(player_id);
    alcazar.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*alcazar.get_id(), Box::new(alcazar));

    let mut unit = SpectralStalker::new(player_id);
    unit.set_zone(Zone::Location(Location::Square(20, Region::Void)));
    state.cards.insert(*unit.get_id(), Box::new(unit.clone()));

    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let zones = unit
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(zones.contains(&Zone::Location(Location::Square(20, Region::Void))));
    assert!(!zones.contains(&Zone::Location(Location::Square(1, Region::Void))));
}

#[tokio::test]
async fn test_great_wall_blocks_enemy_ground_movement_through_top_border() {
    let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
    let wall_owner = state.players[0].id;
    let enemy_id = state.players[1].id;

    let mut wall = GreatWall::new(wall_owner);
    wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*wall.get_id(), Box::new(wall));

    let mut enemy = ApprenticeWizard::new(enemy_id);
    enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    state.cards.insert(*enemy.get_id(), Box::new(enemy.clone()));

    let zones = enemy
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(!zones.contains(&Zone::Location(Location::Square(8, Region::Surface))));
}

#[tokio::test]
async fn test_great_wall_blocks_enemy_ground_movement_out_through_top_border() {
    let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
    let wall_owner = state.players[0].id;
    let enemy_id = state.players[1].id;

    let mut wall = GreatWall::new(wall_owner);
    wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*wall.get_id(), Box::new(wall));

    let mut enemy = ApprenticeWizard::new(enemy_id);
    enemy.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*enemy.get_id(), Box::new(enemy.clone()));

    let zones = enemy
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(!zones.contains(&Zone::Location(Location::Square(13, Region::Surface))));
}

#[tokio::test]
async fn test_great_wall_allows_allied_and_airborne_movement_through_top_border() {
    let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
    let wall_owner = state.players[0].id;
    let enemy_id = state.players[1].id;

    let mut wall = GreatWall::new(wall_owner);
    wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*wall.get_id(), Box::new(wall));

    let mut ally = ApprenticeWizard::new(wall_owner);
    ally.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    state.cards.insert(*ally.get_id(), Box::new(ally.clone()));

    let ally_zones = ally
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(ally_zones.contains(&Zone::Location(Location::Square(8, Region::Surface))));

    let mut airborne_enemy = ApprenticeWizard::new(enemy_id);
    airborne_enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    airborne_enemy.add_ability(Ability::Airborne);
    state
        .cards
        .insert(*airborne_enemy.get_id(), Box::new(airborne_enemy.clone()));

    let airborne_enemy_zones = airborne_enemy
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    assert!(airborne_enemy_zones.contains(&Zone::Location(Location::Square(8, Region::Surface))));
}

#[tokio::test]
async fn test_great_wall_blocks_paths_through_top_border() {
    let mut state = State::new_mock_state(vec![3, 7, 9, 13]);
    let wall_owner = state.players[0].id;
    let enemy_id = state.players[1].id;

    let mut wall = GreatWall::new(wall_owner);
    wall.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*wall.get_id(), Box::new(wall));

    let mut enemy = ApprenticeWizard::new(enemy_id);
    enemy.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    enemy.add_ability(Ability::Movement(1));
    state.cards.insert(*enemy.get_id(), Box::new(enemy.clone()));

    let paths = enemy
        .get_valid_move_paths(
            &state,
            &Zone::Location(Location::Square(3, Region::Surface)),
        )
        .await
        .expect("paths to be computed");
    assert!(paths.is_empty());
}

#[test]
fn test_additional_cost_two_taps() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let cost = Cost::ZERO
        .clone()
        .with_additional(AdditionalCost::tap(
            CardQuery::new()
                .untapped()
                .units()
                .in_location(Location::Square(10, Region::Surface)),
        ))
        .with_additional(AdditionalCost::tap(
            CardQuery::new()
                .untapped()
                .units()
                .in_location(Location::Square(10, Region::Surface)),
        ));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "no units in the zone");

    let mut unit = ApprenticeWizard::new(player_id);
    let unit_id = *unit.get_id();
    unit.set_zone(Zone::Location(Location::Square(10, Region::Surface)));
    state.cards.insert(*unit.get_id(), Box::new(unit));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "only one unit in the zone, two are required");

    let mut unit = ApprenticeWizard::new(player_id);
    unit.set_zone(Zone::Location(Location::Square(10, Region::Surface)));
    state.cards.insert(*unit.get_id(), Box::new(unit));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(can_afford, "two untapped units the zone");

    let unit = state.get_card_mut(&unit_id);
    unit.set_tapped(true);
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "only one untapped unit in the zone");
}

#[tokio::test]
async fn test_get_valid_move_paths_movement_plus_1() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(
            &state,
            &Zone::Location(Location::Square(14, Region::Surface)),
        )
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 2, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface))
    ]));
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface))
    ]));
}

#[tokio::test]
async fn test_get_valid_move_paths_movement_plus_1_airborne() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Airborne);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(
            &state,
            &Zone::Location(Location::Square(14, Region::Surface)),
        )
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 3 valid paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface))
    ]));
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface))
    ]));
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface))
    ]));
}

#[tokio::test]
async fn test_get_valid_move_paths_movement_plus_2() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Movement(2));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(
            &state,
            &Zone::Location(Location::Square(15, Region::Surface)),
        )
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(10, Region::Surface)),
        Zone::Location(Location::Square(15, Region::Surface))
    ]));
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface)),
        Zone::Location(Location::Square(15, Region::Surface))
    ]));
    assert!(paths.contains(&vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface)),
        Zone::Location(Location::Square(15, Region::Surface))
    ]));
}

#[tokio::test]
async fn test_get_valid_move_zones_basic_movement() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_movement_plus_1() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Movement(1));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
        Zone::Location(Location::Square(6, Region::Surface)),
        Zone::Location(Location::Square(10, Region::Surface)),
        Zone::Location(Location::Square(12, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_basic_movement_with_voids() {
    let mut state = State::new_mock_state(vec![3, 8, 9]);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_movement_plus_1_with_voids() {
    let zones_with_sites = vec![2, 3, 4, 8, 9, 12, 13];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Movement(1));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(12, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_basic_movement_with_voidwalk() {
    let zones_with_sites = vec![3, 8, 9];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Voidwalk);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_airborne() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Airborne);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(12, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_airborne_with_voids() {
    let zones_with_sites = vec![2, 3, 4, 8, 9, 12, 13];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Airborne);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();

    let mut expected = vec![
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(12, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[tokio::test]
async fn test_get_valid_move_zones_airborne_and_voidwalk() {
    let zones_with_sites = vec![2, 3, 4, 7, 8, 9, 12, 13, 14];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Airborne);
    card.add_ability(Ability::Voidwalk);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(9, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(12, Region::Surface)),
        Zone::Location(Location::Square(14, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_play_zones_site_second_site() {
    let zones_with_sites = vec![3];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = AridDesert::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let mut zones = card
        .get_valid_play_zones(&state, &player_id, &avatar_id)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(4, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_awakened_mummies_can_be_played_underground_at_land_sites() {
    let mut state = State::new_mock_state(vec![1, 2]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 1;

    let water_site_id = state
        .cards
        .values()
        .find(|card| {
            card.is_site()
                && *card.get_zone() == Zone::Location(Location::Square(2, Region::Surface))
        })
        .map(|card| *card.get_id())
        .expect("mock site should exist");
    state.cards.remove(&water_site_id);

    let mut water_site = SpringRiver::new(player_id);
    water_site.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state
        .cards
        .insert(*water_site.get_id(), Box::new(water_site));

    let mut card = AwakenedMummies::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let zones = card
        .get_valid_play_zones(&state, &player_id, &avatar_id)
        .expect("zones to be computed");

    assert!(zones.contains(&Zone::Location(Location::Square(1, Region::Underground))));
    assert!(!zones.contains(&Zone::Location(Location::Square(2, Region::Underground))));
    assert!(zones.iter().all(|zone| {
        matches!(
            zone,
            Zone::Location(Location::Square(_, Region::Underground))
        )
    }));
}

#[test]
fn test_burrowing_minion_can_be_played_surface_or_underground() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;

    let mut site = SimpleVillage::new(player_id);
    site.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(*site.get_id(), Box::new(site));

    let mut card = RootSpider::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let zones = card
        .get_valid_play_zones(&state, &player_id, &avatar_id)
        .expect("zones to be computed");

    assert!(zones.contains(&Zone::Location(Location::Square(1, Region::Surface))));
    assert!(zones.contains(&Zone::Location(Location::Square(1, Region::Underground))));
}

#[test]
fn test_auras_can_be_played_at_any_surface_intersection() {
    let mut state = State::new_mock_state(vec![1, 2]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 4;

    let mut card = Drought::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let mut zones = card
        .get_valid_play_zones(&state, &player_id, &avatar_id)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = Zone::all_intersections();
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_auras_can_only_be_played_surface_or_void() {
    let mut state = State::new_mock_state(vec![1, 2]);
    let player_id = state.players[0].id;

    let mut card = Drought::new(player_id);
    card.set_zone(Zone::Hand);
    let card_id = *card.get_id();
    state.cards.insert(card_id, Box::new(card));

    assert!(
        Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface))
            .is_valid_play_zone_for(&state, &card_id, &player_id)
            .expect("surface intersection should validate")
    );
    assert!(
        Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Void))
            .is_valid_play_zone_for(&state, &card_id, &player_id)
            .expect("void intersection should validate")
    );
    assert!(
        !Zone::Location(Location::Intersection(
            vec![1, 2, 6, 7],
            Region::Underground
        ))
        .is_valid_play_zone_for(&state, &card_id, &player_id)
        .expect("underground intersection should validate")
    );
    assert!(
        !Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Underwater))
            .is_valid_play_zone_for(&state, &card_id, &player_id)
            .expect("underwater intersection should validate")
    );
}

#[test]
fn test_can_afford_cost() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 2;

    let mut card = OgreGoons::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let can_afford = card
        .get_costs(&state)
        .unwrap()
        .can_afford(&state, player_id)
        .unwrap();
    assert!(!can_afford);

    *state.get_player_mana_mut(&player_id) = 3;
    let can_afford = card
        .get_costs(&state)
        .unwrap()
        .can_afford(&state, player_id)
        .unwrap();
    assert!(!can_afford);

    let mut arid_desert = AridDesert::new(player_id);
    arid_desert.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .cards
        .insert(*arid_desert.get_id(), Box::new(arid_desert));

    // The player now has 3 mana and a fire affinity of 1, so they should be able to afford the
    // Ogre Goons in their hand, which costs 3F.
    let can_afford = card
        .get_costs(&state)
        .unwrap()
        .can_afford(&state, player_id)
        .unwrap();
    assert!(can_afford);
}
