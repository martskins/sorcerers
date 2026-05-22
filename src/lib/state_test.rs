use crate::{
    card::{
        Ability, AridDesert, BeastOfBurden, Card, CauldronCrones, CourtesanThais, DonnybrookInn,
        Enchantress, FootSoldier, HeadlessHaunt, KiteArcher, NimbusJinn, Region, RimlandNomads,
        Rubble, from_name_and_zone,
    },
    deck::Deck,
    effect::Effect,
    game::{NO_CONTROLLER, Thresholds},
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, QueryCache, ZoneQuery},
    state::{Player, PlayerWithDeck, State, TemporaryEffect, Turn, TurnIterator},
    zone::Zone,
};

fn setup_carrying_state() -> (State, async_channel::Receiver<ServerMessage>) {
    QueryCache::init();

    let player_one_id = uuid::Uuid::new_v4();
    let player_two_id = uuid::Uuid::new_v4();

    let avatar_one = Enchantress::new(player_one_id);
    let avatar_one_id = *avatar_one.get_id();
    let avatar_two = Enchantress::new(player_two_id);
    let avatar_two_id = *avatar_two.get_id();

    let zones = Zone::all_realm();
    let mut p1_cards: Vec<Box<dyn Card>> = zones
        .iter()
        .map(|z| from_name_and_zone(AridDesert::NAME, &player_one_id, z.clone()))
        .collect();
    p1_cards.push(Box::new(avatar_one));

    let player1 = PlayerWithDeck {
        player: Player {
            id: player_one_id,
            name: "Player 1".to_string(),
        },
        deck: Deck::new(
            &player_one_id,
            "Test Deck".to_string(),
            vec![],
            vec![],
            avatar_one_id,
        ),
        cards: p1_cards,
    };

    let player2 = PlayerWithDeck {
        player: Player {
            id: player_two_id,
            name: "Player 2".to_string(),
        },
        deck: Deck::new(
            &player_two_id,
            "Test Deck".to_string(),
            vec![],
            vec![],
            avatar_two_id,
        ),
        cards: vec![Box::new(avatar_two)],
    };

    let (server_tx, server_rx) = async_channel::unbounded();
    let (_, client_rx) = async_channel::unbounded();
    let state = State::new(
        uuid::Uuid::new_v4(),
        vec![player1, player2],
        server_tx,
        client_rx,
    );
    (state, server_rx)
}

#[tokio::test]
async fn test_carried_minion_follows_carrier() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(1, Region::Surface));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(1, Region::Surface));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move carrier to Realm(2)
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: carrier_id,
        from: Zone::Location(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Location(2, Region::Surface)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(2, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(2, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_bearer_id().unwrap(),
        Some(carrier_id)
    );
}

#[tokio::test]
async fn test_carried_minion_moves_independently_and_clears_bearer() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(1, Region::Surface));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(1, Region::Surface));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move passenger to Realm(2) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Location(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Location(2, Region::Surface)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(1, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(2, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_bearer_id().unwrap(),
        None,
        "Passenger should no longer be carried after moving independently"
    );
}

#[tokio::test]
async fn test_carried_minion_moves_independently_through_path_and_clears_bearer() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(1, Region::Surface));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(1, Region::Surface));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move passenger through Realm(2) to Realm(3) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Location(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Location(3, Region::Surface)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: Some(vec![
            Zone::Location(1, Region::Surface),
            Zone::Location(2, Region::Surface),
            Zone::Location(3, Region::Surface),
        ]),
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(1, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(3, Region::Surface)
    );
    assert_eq!(
        state.get_card(&passenger_id).get_bearer_id().unwrap(),
        None,
        "Passenger should no longer be carried after moving independently through path"
    );
}

#[tokio::test]
async fn test_conferred_abilities() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(1, Region::Surface));
    carrier.add_ability(Ability::Airborne);
    carrier.add_ability(Ability::Voidwalk);
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(1, Region::Surface));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    let passenger_abilities = state.get_card(&passenger_id).get_abilities(&state).unwrap();
    assert!(passenger_abilities.contains(&Ability::Airborne));
    assert!(passenger_abilities.contains(&Ability::Voidwalk));
    assert!(!passenger_abilities.contains(&Ability::Burrowing));
    assert!(!passenger_abilities.contains(&Ability::Submerge));

    // Move passenger away, abilities should be lost
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Location(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Location(2, Region::Surface)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    let passenger_abilities_after = state.get_card(&passenger_id).get_abilities(&state).unwrap();
    assert!(!passenger_abilities_after.contains(&Ability::Airborne));
    assert!(!passenger_abilities_after.contains(&Ability::Voidwalk));
}

#[test]
fn test_inteceptors() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(8, Region::Surface));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(12, Region::Surface));
    state
        .cards
        .insert(*kite_archer.get_id(), Box::new(kite_archer.clone()));

    let path = vec![
        Zone::Location(8, Region::Surface),
        Zone::Location(13, Region::Surface),
        Zone::Location(18, Region::Surface),
    ];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 1);
    assert_eq!(&interceptors[0].0, kite_archer.get_id());
}

#[test]
fn test_no_inteceptors() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(8, Region::Surface));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(11, Region::Surface));
    state
        .cards
        .insert(*kite_archer.get_id(), Box::new(kite_archer.clone()));

    let path = vec![
        Zone::Location(8, Region::Surface),
        Zone::Location(13, Region::Surface),
        Zone::Location(18, Region::Surface),
    ];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_voidwalking_interceptor() {
    let mut state = State::new_mock_state(vec![8, 13, 18]);
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(8, Region::Surface));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = HeadlessHaunt::new(opponent_id);
    headless_haunt.set_zone(Zone::Location(12, Region::Surface));
    state
        .cards
        .insert(*headless_haunt.get_id(), Box::new(headless_haunt.clone()));

    let path = vec![
        Zone::Location(8, Region::Surface),
        Zone::Location(13, Region::Surface),
        Zone::Location(18, Region::Surface),
    ];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 1);
}

#[test]
fn test_airborne_interceptor() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(8, Region::Surface));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = NimbusJinn::new(opponent_id);
    headless_haunt.set_zone(Zone::Location(12, Region::Surface));
    state
        .cards
        .insert(*headless_haunt.get_id(), Box::new(headless_haunt.clone()));

    let path = vec![
        Zone::Location(8, Region::Surface),
        Zone::Location(13, Region::Surface),
        Zone::Location(18, Region::Surface),
    ];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 3);
}

#[tokio::test]
async fn test_get_effective_costs_donnybrook_inn() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;
    let mut donnybrook_inn = DonnybrookInn::new(player_id);
    donnybrook_inn.set_zone(Zone::Location(3, Region::Surface));
    state
        .cards
        .insert(*donnybrook_inn.get_id(), Box::new(donnybrook_inn.clone()));

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state
        .cards
        .insert(*cauldron_crones.get_id(), Box::new(cauldron_crones.clone()));

    state.compute_world_effects().await.unwrap();
    let regular_costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(regular_costs.mana_value(), 3);

    let inn_costs = state
        .get_effective_costs(
            cauldron_crones.get_id(),
            Some(donnybrook_inn.get_zone()),
            &player_id,
        )
        .unwrap();
    assert_eq!(inn_costs.mana_value(), 2);
}

#[tokio::test]
async fn test_get_effective_costs_ignoring_thresholds() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state
        .cards
        .insert(*cauldron_crones.get_id(), Box::new(cauldron_crones.clone()));

    state.compute_world_effects().await.unwrap();
    let regular_costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(regular_costs.mana_value(), 3);
    assert_eq!(regular_costs.thresholds_cost(), &Thresholds::parse("F"));

    state
        .temporary_effects_mut()
        .push(TemporaryEffect::IgnoreCostThresholds {
            affected_cards: std::convert::Into::<CardQuery>::into(cauldron_crones.get_id())
                .including_not_in_play(),
            expires_on_effect: EffectQuery::TurnEnd { player_id: None },
            for_player: player_id,
        });
    let costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(costs.mana_value(), 3);
    assert_eq!(costs.thresholds_cost(), &Thresholds::ZERO);
}

#[test]
fn test_state_aware_nearby_locations_do_not_create_surface_void_locations() {
    let state = State::new_mock_state(vec![8, 9]);

    let source = Zone::Location(8, Region::Surface);

    assert!(
        source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(9, Region::Surface))
    );
    assert!(
        !source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(13, Region::Surface))
    );
    assert!(
        source
            .get_adjacent_voids(&state)
            .contains(&Zone::Location(13, Region::Void))
    );
    assert!(
        !source
            .get_adjacent_voids(&state)
            .contains(&Zone::Location(9, Region::Void))
    );
}

#[test]
fn test_zone_query_adjacent_to_uses_state_aware_locations() {
    let state = State::new_mock_state(vec![8, 9]);
    let source = Zone::Location(13, Region::Void);

    let options = ZoneQuery::new().adjacent_to(&source).options(&state);

    assert!(options.contains(&Zone::Location(13, Region::Void)));
    assert!(!options.contains(&Zone::Location(8, Region::Surface)));
}

#[test]
fn test_card_query_adjacent_to_uses_state_aware_locations() {
    let mut state = State::new_mock_state(vec![8, 9]);
    let player_id = state.players[0].id;
    let source = Zone::Location(8, Region::Surface);

    let mut foot_soldier = FootSoldier::new(player_id);
    foot_soldier.set_zone(Zone::Location(13, Region::Surface));
    let foot_soldier_id = *foot_soldier.get_id();
    state.cards.insert(foot_soldier_id, Box::new(foot_soldier));

    assert!(
        !CardQuery::new()
            .minions()
            .adjacent_to(&source)
            .matches(&foot_soldier_id, &state)
    );
}

#[test]
fn test_adjacent_sites_cross_region_boundaries() {
    let state = State::new_mock_state(vec![8]);

    let source = Zone::Location(13, Region::Void);

    assert!(
        !source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(8, Region::Surface))
    );
    assert!(
        source
            .get_adjacent_sites(&state)
            .contains(&Zone::Location(8, Region::Surface))
    );
}

#[test]
fn test_card_query_spatial_filters_resolve_with_current_state() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let source = Zone::Location(13, Region::Void);
    let query = CardQuery::new().sites().adjacent_sites_to(&source);

    let mut site = AridDesert::new(player_id);
    site.set_zone(Zone::Location(8, Region::Surface));
    let site_id = *site.get_id();
    state.cards.insert(site_id, Box::new(site));

    assert!(query.matches(&site_id, &state));
}

#[test]
fn test_rubble_has_no_controller() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut rubble = Rubble::new(player_id);
    let rubble_id = *rubble.get_id();
    rubble.set_zone(Zone::Location(8, Region::Surface));
    state.cards.insert(rubble_id, Box::new(rubble));

    assert_eq!(
        state.get_card(&rubble_id).get_controller_id(&state),
        NO_CONTROLLER
    );
    assert!(!CardQuery::new()
        .sites()
        .controlled_by(&player_id)
        .all(&state)
        .contains(&rubble_id));
    assert!(!CardQuery::new()
        .sites()
        .controlled_by(&opponent_id)
        .all(&state)
        .contains(&rubble_id));
    assert_eq!(state.get_thresholds_for_player(&player_id), Thresholds::ZERO);
    assert_eq!(state.get_thresholds_for_player(&opponent_id), Thresholds::ZERO);
}

#[test]
fn test_turn_iterator() {
    let player_one = uuid::Uuid::new_v4();
    let player_two = uuid::Uuid::new_v4();
    let mut it = TurnIterator::new(vec![player_one, player_two]);

    assert_eq!(it.current().player_id, player_one);

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_two);

    it.override_next(Turn::controlled_by(player_two, player_one));

    let curr = it.next();
    assert!(curr.is_some());
    let curr = curr.unwrap();
    assert_eq!(curr.player_id, player_two);
    assert_eq!(curr.controller_override, Some(player_one));

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_one);
}

#[tokio::test]
async fn test_courtesan_thais_genesis_overrides_next_turns() {
    let (mut state, _rx) = setup_carrying_state();
    let player_one = state.players[0].id;
    let player_two = state.players[1].id;
    let thais = CourtesanThais::new(player_one);

    let effects = thais.genesis(&state).await.unwrap();
    state.queue(effects);
    state.apply_effects_without_log().await.unwrap();

    let next_turn = state.next_turn();
    assert_eq!(next_turn.player_id(), player_two);
    assert_eq!(next_turn.controller_override(), Some(player_one));

    state.advance_turn();
    assert_eq!(state.current_player(), player_two);
    assert_eq!(state.current_turn_controller(), player_one);

    match state.into_sync().unwrap() {
        ServerMessage::Sync { current_player, .. } => {
            assert_eq!(current_player, player_one);
        }
        _ => unreachable!(),
    }

    let next_turn = state.next_turn();
    assert_eq!(next_turn.player_id(), player_one);
    assert_eq!(next_turn.controller_override(), Some(player_two));
}
