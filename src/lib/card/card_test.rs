use crate::{
    card::{
        Ability, AdditionalCost, ApprenticeWizard, AridDesert, AssortedAnimals, Card,
        CauldronCrones, Cost, CostOptions, Drought, ManaCost, OgreGoons, PayableCost, Region,
        RimlandNomads, RootSpider, SimpleVillage, SpringRiver,
    },
    game::Thresholds,
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
    state.add_card(Box::new(unit));
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
fn test_alternative_additional_only_cost_does_not_change_printed_mana_cost() {
    let costs = CostOptions::basic(3, "FF").with_alternative(PayableCost::additional_only(
        AdditionalCost::discard(CardQuery::new().minions()),
    ));

    assert_eq!(costs.printed_mana_value(), Some(3));
    assert_eq!(costs.printed_thresholds(), &Thresholds::parse("FF"));

    let payable_options = costs.payable_options();
    assert_eq!(payable_options[0].payable_mana_value(), Some(3));
    assert_eq!(payable_options[1].payable_mana_value(), Some(0));
}

#[test]
fn test_variable_costs_expose_variable_printed_mana_cost() {
    let costs = CostOptions::variable("EE");

    assert_eq!(costs.printed_mana_cost(), &ManaCost::Variable);
    assert_eq!(costs.printed_mana_value(), None);
}

#[test]
fn test_card_query_mana_cost_filter_uses_printed_mana_value() {
    let mut state = State::new_mock_state(Vec::new());
    let player_id = state.players[0].id;

    let cauldron_crones = CauldronCrones::new(player_id);
    let cauldron_crones_id = *cauldron_crones.get_id();
    state.add_card(Box::new(cauldron_crones));

    let assorted_animals = AssortedAnimals::new(player_id);
    let assorted_animals_id = *assorted_animals.get_id();
    state.add_card(Box::new(assorted_animals));

    let matching_cards = CardQuery::new()
        .including_not_in_play()
        .with_mana_cost(3)
        .all(&state);
    assert!(matching_cards.contains(&cauldron_crones_id));
    assert!(!matching_cards.contains(&assorted_animals_id));
}

#[test]
fn test_card_query_near_to_stays_in_origin_region() {
    let mut land_state = State::new_mock_state(vec![7, 8]);
    let player_id = land_state.players[0].id;

    let mut surface_unit = ApprenticeWizard::new(player_id);
    let surface_unit_id = *surface_unit.get_id();
    surface_unit.set_zone(Zone::Location(Location::Square(7, Region::Surface)));
    land_state.add_card(Box::new(surface_unit));

    let mut underground_unit = ApprenticeWizard::new(player_id);
    let underground_unit_id = *underground_unit.get_id();
    underground_unit.set_zone(Zone::Location(Location::Square(7, Region::Underground)));
    land_state.add_card(Box::new(underground_unit));

    let surface_matches = CardQuery::new()
        .minions()
        .near_to(&Location::Square(8, Region::Surface))
        .all(&land_state);
    assert!(surface_matches.contains(&surface_unit_id));
    assert!(!surface_matches.contains(&underground_unit_id));

    let underground_matches = CardQuery::new()
        .minions()
        .near_to(&Location::Square(8, Region::Underground))
        .all(&land_state);
    assert!(!underground_matches.contains(&surface_unit_id));
    assert!(underground_matches.contains(&underground_unit_id));

    let mut water_state = State::new_mock_state(Vec::new());
    let player_id = water_state.players[0].id;

    let mut origin_site = SpringRiver::new(player_id);
    origin_site.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    water_state.add_card(Box::new(origin_site));

    let mut nearby_site = SpringRiver::new(player_id);
    nearby_site.set_zone(Zone::Location(Location::Square(7, Region::Surface)));
    water_state.add_card(Box::new(nearby_site));

    let mut surface_unit = ApprenticeWizard::new(player_id);
    let surface_unit_id = *surface_unit.get_id();
    surface_unit.set_zone(Zone::Location(Location::Square(7, Region::Surface)));
    water_state.add_card(Box::new(surface_unit));

    let mut underwater_unit = ApprenticeWizard::new(player_id);
    let underwater_unit_id = *underwater_unit.get_id();
    underwater_unit.set_zone(Zone::Location(Location::Square(7, Region::Underwater)));
    water_state.add_card(Box::new(underwater_unit));

    let underwater_matches = CardQuery::new()
        .minions()
        .near_to(&Location::Square(8, Region::Underwater))
        .all(&water_state);
    assert!(!underwater_matches.contains(&surface_unit_id));
    assert!(underwater_matches.contains(&underwater_unit_id));
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
    state.add_card(Box::new(unit));
    let can_afford = cost
        .can_afford(&state, player_id)
        .expect("should not error");
    assert!(!can_afford, "only one unit in the zone, two are required");

    let mut unit = ApprenticeWizard::new(player_id);
    unit.set_zone(Zone::Location(Location::Square(10, Region::Surface)));
    state.add_card(Box::new(unit));
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
    state.add_card(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Location::Square(14, Region::Surface))
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 2, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(14, Region::Surface)
    ]));
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(14, Region::Surface)
    ]));
}

#[tokio::test]
async fn test_get_valid_move_paths_movement_plus_1_airborne() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Airborne);
    state.add_card(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Location::Square(14, Region::Surface))
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 3 valid paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(14, Region::Surface)
    ]));
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(14, Region::Surface)
    ]));
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(14, Region::Surface)
    ]));
}

#[tokio::test]
async fn test_get_valid_move_paths_movement_plus_2() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = RimlandNomads::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    card.add_ability(Ability::Movement(2));
    state.add_card(Box::new(card.clone()));

    let paths = card
        .get_valid_move_paths(&state, &Location::Square(15, Region::Surface))
        .await
        .expect("paths to be computed");
    assert_eq!(paths.len(), 3, "Expected 2 paths, got {:?}", paths);
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(10, Region::Surface),
        Location::Square(15, Region::Surface)
    ]));
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(14, Region::Surface),
        Location::Square(15, Region::Surface)
    ]));
    assert!(paths.contains(&vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(14, Region::Surface),
        Location::Square(15, Region::Surface)
    ]));
}

#[tokio::test]
async fn test_get_valid_move_zones_basic_movement() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(7, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(13, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(7, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
        Location::Square(6, Region::Surface),
        Location::Square(10, Region::Surface),
        Location::Square(12, Region::Surface),
        Location::Square(14, Region::Surface),
        Location::Square(2, Region::Surface),
        Location::Square(4, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(2, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(4, Region::Surface),
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(12, Region::Surface),
        Location::Square(13, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(7, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(13, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(7, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(12, Region::Surface),
        Location::Square(14, Region::Surface),
        Location::Square(2, Region::Surface),
        Location::Square(4, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();

    let mut expected = vec![
        Location::Square(2, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(4, Region::Surface),
        Location::Square(8, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(12, Region::Surface),
        Location::Square(13, Region::Surface),
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
    state.add_card(Box::new(card.clone()));

    let mut zones = card
        .get_valid_move_locations(&state)
        .await
        .expect("zones to be computed");
    zones.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(7, Region::Surface),
        Location::Square(9, Region::Surface),
        Location::Square(3, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(12, Region::Surface),
        Location::Square(14, Region::Surface),
        Location::Square(2, Region::Surface),
        Location::Square(4, Region::Surface),
    ];
    expected.sort();
    assert_eq!(zones, expected);
}

#[test]
fn test_get_valid_play_locations_site_second_site() {
    let zones_with_sites = vec![3];
    let mut state = State::new_mock_state(zones_with_sites);
    let player_id = state.players[0].id;
    let mut card = AridDesert::new(player_id);
    card.set_zone(Zone::Hand);
    state.add_card(Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let mut locations = card
        .get_valid_play_locations(&state, &player_id, &avatar_id)
        .expect("zones to be computed");
    locations.sort();
    let mut expected = vec![
        Location::Square(8, Region::Surface),
        Location::Square(4, Region::Surface),
        Location::Square(2, Region::Surface),
    ];
    expected.sort();
    assert_eq!(locations, expected);
}

#[test]
fn test_burrowing_minion_can_be_played_surface_or_underground() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;

    let mut site = SimpleVillage::new(player_id);
    site.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(site));

    let mut card = RootSpider::new(player_id);
    card.set_zone(Zone::Hand);
    state.add_card(Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let locations = card
        .get_valid_play_locations(&state, &player_id, &avatar_id)
        .expect("zones to be computed");

    assert!(locations.contains(&Location::Square(1, Region::Surface)));
    assert!(locations.contains(&Location::Square(1, Region::Underground)));
}

#[test]
fn test_auras_can_be_played_at_any_surface_intersection() {
    let mut state = State::new_mock_state(vec![1, 2]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 4;

    let mut card = Drought::new(player_id);
    card.set_zone(Zone::Hand);
    state.add_card(Box::new(card.clone()));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    let mut locations = card
        .get_valid_play_locations(&state, &player_id, &avatar_id)
        .expect("zones to be computed");
    locations.sort();
    let mut expected = Location::all_intersections();
    expected.sort();
    assert_eq!(locations, expected);
}

#[test]
fn test_auras_can_only_be_played_surface_or_void() {
    let mut state = State::new_mock_state(vec![1, 2]);
    let player_id = state.players[0].id;

    let mut card = Drought::new(player_id);
    card.set_zone(Zone::Hand);
    let card_id = *card.get_id();
    state.add_card(Box::new(card));

    assert!(
        Location::Intersection(vec![1, 2, 6, 7], Region::Surface)
            .is_valid_play_location_for(&state, &card_id, &player_id)
            .expect("surface intersection should validate")
    );
    assert!(
        Location::Intersection(vec![1, 2, 6, 7], Region::Void)
            .is_valid_play_location_for(&state, &card_id, &player_id)
            .expect("void intersection should validate")
    );
    assert!(
        !Location::Intersection(vec![1, 2, 6, 7], Region::Underground)
            .is_valid_play_location_for(&state, &card_id, &player_id)
            .expect("underground intersection should validate")
    );
    assert!(
        !Location::Intersection(vec![1, 2, 6, 7], Region::Underwater)
            .is_valid_play_location_for(&state, &card_id, &player_id)
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
    state.add_card(Box::new(card.clone()));

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
    state.add_card(Box::new(arid_desert));

    // The player now has 3 mana and a fire affinity of 1, so they should be able to afford the
    // Ogre Goons in their hand, which costs 3F.
    let can_afford = card
        .get_costs(&state)
        .unwrap()
        .can_afford(&state, player_id)
        .unwrap();
    assert!(can_afford);
}
