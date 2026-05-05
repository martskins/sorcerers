use crate::{
    card::{
        Ability, AdditionalCost, ApprenticeWizard, AridDesert, Card, Cost, OgreGoons,
        RimlandNomads, Zone,
    },
    state::{CardQuery, State},
};

#[test]
fn test_additional_cost_tap() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let cost = Cost::additional_only(AdditionalCost::tap(
        CardQuery::new()
            .untapped()
            .units()
            .in_zone(&Zone::Realm(10)),
    ));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "no units in the zone");

    let mut unit = ApprenticeWizard::new(player_id);
    let unit_id = *unit.get_id();
    unit.set_zone(Zone::Realm(10));
    state.cards.push(Box::new(unit));
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

#[test]
fn test_additional_cost_two_taps() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let cost = Cost::ZERO
        .clone()
        .with_additional(AdditionalCost::tap(
            CardQuery::new()
                .untapped()
                .units()
                .in_zone(&Zone::Realm(10)),
        ))
        .with_additional(AdditionalCost::tap(
            CardQuery::new()
                .untapped()
                .units()
                .in_zone(&Zone::Realm(10)),
        ));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "no units in the zone");

    let mut unit = ApprenticeWizard::new(player_id);
    let unit_id = *unit.get_id();
    unit.set_zone(Zone::Realm(10));
    state.cards.push(Box::new(unit));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "only one unit in the zone, two are required");

    let mut unit = ApprenticeWizard::new(player_id);
    unit.set_zone(Zone::Realm(10));
    state.cards.push(Box::new(unit));
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

#[test]
fn test_get_valid_move_paths_movement_plus_1() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Zone::Realm(14))
        .expect("paths to be computed");
    assert_eq!(paths.len(), 2, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(14)]));
    assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(14)]));
}

#[test]
fn test_get_valid_move_paths_movement_plus_1_airborne() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Airborne);
    state.cards.push(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Zone::Realm(14))
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 3 valid paths, got {:?}", paths);
    assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(14)]));
    assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(14)]));
    assert!(paths.contains(&vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(14)]));
}

#[test]
fn test_get_valid_move_paths_movement_plus_2() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Movement(2));
    state.cards.push(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Zone::Realm(15))
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(10),
        Zone::Realm(15)
    ]));
    assert!(paths.contains(&vec![
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(14),
        Zone::Realm(15)
    ]));
    assert!(paths.contains(&vec![
        Zone::Realm(8),
        Zone::Realm(13),
        Zone::Realm(14),
        Zone::Realm(15)
    ]));
}

#[test]
fn test_get_valid_move_zones_basic_movement() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(8),
        Zone::Realm(7),
        Zone::Realm(9),
        Zone::Realm(3),
        Zone::Realm(13),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_movement_plus_1() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Movement(1));
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(8),
        Zone::Realm(7),
        Zone::Realm(9),
        Zone::Realm(3),
        Zone::Realm(13),
        Zone::Realm(18),
        Zone::Realm(6),
        Zone::Realm(10),
        Zone::Realm(12),
        Zone::Realm(14),
        Zone::Realm(2),
        Zone::Realm(4),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_basic_movement_with_voids() {
    let zones_with_sites = vec![Zone::Realm(3), Zone::Realm(8), Zone::Realm(9)];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![Zone::Realm(8), Zone::Realm(9), Zone::Realm(3)];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_movement_plus_1_with_voids() {
    let zones_with_sites = vec![
        Zone::Realm(2),
        Zone::Realm(3),
        Zone::Realm(4),
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(12),
        Zone::Realm(13),
    ];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Movement(1));
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(2),
        Zone::Realm(3),
        Zone::Realm(4),
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(12),
        Zone::Realm(13),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_basic_movement_with_voidwalk() {
    let zones_with_sites = vec![Zone::Realm(3), Zone::Realm(8), Zone::Realm(9)];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Voidwalk);
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(8),
        Zone::Realm(7),
        Zone::Realm(9),
        Zone::Realm(3),
        Zone::Realm(13),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_airborne() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Airborne);
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(8),
        Zone::Realm(7),
        Zone::Realm(9),
        Zone::Realm(3),
        Zone::Realm(13),
        Zone::Realm(12),
        Zone::Realm(14),
        Zone::Realm(2),
        Zone::Realm(4),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_airborne_with_voids() {
    let zones_with_sites = vec![
        Zone::Realm(2),
        Zone::Realm(3),
        Zone::Realm(4),
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(12),
        Zone::Realm(13),
    ];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Airborne);
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();

    let mut expected = vec![
        Zone::Realm(2),
        Zone::Realm(3),
        Zone::Realm(4),
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(12),
        Zone::Realm(13),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_move_zones_airborne_and_voidwalk() {
    let zones_with_sites = vec![
        Zone::Realm(2),
        Zone::Realm(3),
        Zone::Realm(4),
        Zone::Realm(7),
        Zone::Realm(8),
        Zone::Realm(9),
        Zone::Realm(12),
        Zone::Realm(13),
        Zone::Realm(14),
    ];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Realm(8));
    card.add_ability(Ability::Airborne);
    card.add_ability(Ability::Voidwalk);
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_zones(&state)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Zone::Realm(8),
        Zone::Realm(7),
        Zone::Realm(9),
        Zone::Realm(3),
        Zone::Realm(13),
        Zone::Realm(12),
        Zone::Realm(14),
        Zone::Realm(2),
        Zone::Realm(4),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_play_zones_site_second_site() {
    let zones_with_sites = vec![Zone::Realm(3)];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = AridDesert::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.push(Box::new(card.clone()));

    let mut zones = card
        .get_valid_play_zones(&state, &player_id)
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![Zone::Realm(8), Zone::Realm(4), Zone::Realm(2)];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_can_afford_cost() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 2;

    let mut card = OgreGoons::new(player_id);
    card.set_zone(Zone::Hand);
    state.cards.push(Box::new(card.clone()));

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
    arid_desert.set_zone(Zone::Realm(3));
    state.cards.push(Box::new(arid_desert));

    // The player now has 3 mana and a fire affinity of 1, so they should be able to afford the
    // Ogre Goons in their hand, which costs 3F.
    let can_afford = card
        .get_costs(&state)
        .unwrap()
        .can_afford(&state, player_id)
        .unwrap();
    assert!(can_afford);
}
