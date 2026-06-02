use crate::{
    card::{
        Ability, AridDesert, BeastOfBurden, Card, CardStatus, CauldronCrones, CourtesanThais,
        DonnybrookInn, Drought, Enchantress, Flood, FootSoldier, FreeCity, HeadlessHaunt,
        KiteArcher, NimbusJinn, Region, RimlandNomads, Rubble, Silence, SistersOfSilence,
        SkyBaron, SmokestacksOfGnaak, SneakThief, UnitBase, from_name_and_zone,
    },
    deck::Deck,
    effect::Effect,
    game::{NO_CONTROLLER, Thresholds},
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, LocationQuery, QueryCache, ZoneQuery},
    state::{
        AbilityRemoval, ContinuousEffect, Player, PlayerWithDeck, State, TemporaryEffect,
        TimedOngoingEffect, Turn, TurnIterator,
    },
    zone::{Location, Zone},
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

async fn insert_realm_card(state: &mut State, mut card: Box<dyn Card>, zone: Zone) -> uuid::Uuid {
    let card_id = *card.get_id();
    card.set_zone(zone);
    state.cards.insert(card_id, card);
    state
        .add_passive_ongoing_effects_for_source(&card_id)
        .await
        .unwrap();
    card_id
}

#[tokio::test]
async fn test_animate_makes_aura_a_minion_until_expiry() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let aura_id = insert_realm_card(
        &mut state,
        Box::new(Silence::new(player_id)),
        Zone::Location(Location::Square(1, Region::Surface)),
    )
    .await;

    Effect::Animate {
        card_id: aura_id,
        unit_base: UnitBase {
            power: 2,
            toughness: 2,
            ..Default::default()
        },
        expires_on_effect: EffectQuery::TurnStart {
            player_id: Some(player_id),
        },
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(state.get_card(&aura_id).is_aura());
    assert!(!state.get_card(&aura_id).is_minion());
    assert!(state.is_minion_card(&aura_id));
    assert!(CardQuery::new().auras().all(&state).contains(&aura_id));
    assert!(CardQuery::new().minions().all(&state).contains(&aura_id));

    Effect::StartTurn { player_id }
        .apply(&mut state)
        .await
        .unwrap();

    assert!(state.get_card(&aura_id).is_aura());
    assert!(!state.get_card(&aura_id).is_minion());
    assert!(!state.is_minion_card(&aura_id));
    assert!(!CardQuery::new().minions().all(&state).contains(&aura_id));
}

#[tokio::test]
async fn test_free_city_is_not_a_unit_before_animation() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let free_city_id = insert_realm_card(
        &mut state,
        Box::new(FreeCity::new(player_id)),
        Zone::Location(Location::Square(1, Region::Surface)),
    )
    .await;

    assert!(state.get_card(&free_city_id).is_site());
    assert!(!state.get_card(&free_city_id).is_unit());
    assert!(!CardQuery::new().units().all(&state).contains(&free_city_id));
}

#[tokio::test]
async fn test_free_city_can_be_chosen_to_defend_before_animation() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let zone = Zone::Location(Location::Square(1, Region::Surface));

    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state.get_card_mut(&avatar_id).set_zone(zone.clone());
    let free_city_id =
        insert_realm_card(&mut state, Box::new(FreeCity::new(player_id)), zone.clone()).await;
    let attacker_id =
        insert_realm_card(&mut state, Box::new(FootSoldier::new(opponent_id)), zone).await;

    assert!(!state.get_card(&free_city_id).is_unit());
    let defenders = state.get_defenders_for_attack(&attacker_id, &avatar_id);
    assert!(
        defenders.contains(&free_city_id),
        "Free City should be a defender candidate before it animates"
    );
}

fn passive_ongoing_timestamps_for_source(state: &State, source_id: &uuid::Uuid) -> Vec<u64> {
    state
        .ongoing_effects
        .iter()
        .filter(|effect| effect.source == Some(*source_id))
        .map(|effect| effect.timestamp)
        .collect()
}

#[tokio::test]
async fn test_carried_minion_follows_carrier() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move carrier to Realm(2)
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: carrier_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(2, Region::Surface)))
                .with_region(crate::card::Region::Surface),
        ),
        tap: false,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(Location::Square(2, Region::Surface))
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(Location::Square(2, Region::Surface))
    );
    assert_eq!(
        state.get_card(&passenger_id).get_bearer_id().unwrap(),
        Some(carrier_id)
    );
}

#[tokio::test]
async fn test_tapping_carrier_does_not_tap_carried_minion() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    Effect::SetTapped {
        card_id: carrier_id,
        tapped: true,
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(state.get_card(&carrier_id).is_tapped());
    assert!(!state.get_card(&passenger_id).is_tapped());
}

#[tokio::test]
async fn test_carried_minion_changes_region_with_carrier() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let mut carrier = BeastOfBurden::new(player_id);
    carrier.add_ability(Ability::Burrowing);
    let carrier_id = *carrier.get_id();
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = RimlandNomads::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    state.queue_one(Effect::SetCardRegion {
        card_id: carrier_id,
        region: Region::Underground,
        tap: false,
    });
    state.apply_effects_without_log().await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_region(&state),
        &Region::Underground
    );
    assert_eq!(
        state.get_card(&passenger_id).get_region(&state),
        &Region::Underground
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Underground))
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
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move passenger to Realm(2) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(2, Region::Surface)))
                .with_region(crate::card::Region::Surface),
        ),
        tap: false,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface))
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(Location::Square(2, Region::Surface))
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
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.insert(passenger_id, Box::new(passenger));

    // Move passenger through Realm(2) to Realm(3) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(3, Region::Surface)))
                .with_region(crate::card::Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Zone::Location(Location::Square(1, Region::Surface)),
            Zone::Location(Location::Square(2, Region::Surface)),
            Zone::Location(Location::Square(3, Region::Surface)),
        ]),
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(
        state.get_card(&carrier_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface))
    );
    assert_eq!(
        state.get_card(&passenger_id).get_zone(),
        &Zone::Location(Location::Square(3, Region::Surface))
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
    carrier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    carrier.add_ability(Ability::Airborne);
    carrier.add_ability(Ability::Voidwalk);
    state.cards.insert(carrier_id, Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
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
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(2, Region::Surface)))
                .with_region(crate::card::Region::Surface),
        ),
        tap: false,
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
    rimland_nomads.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state
        .cards
        .insert(*kite_archer.get_id(), Box::new(kite_archer.clone()));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors =
        state.get_interceptors_for_move(&path, rimland_nomads.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 1);
    assert_eq!(&interceptors[0], kite_archer.get_id());
}

#[test]
fn test_no_inteceptors() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    state
        .cards
        .insert(*kite_archer.get_id(), Box::new(kite_archer.clone()));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors =
        state.get_interceptors_for_move(&path, rimland_nomads.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_tapped_units_cannot_intercept() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    foot_soldier.set_tapped(true);
    state
        .cards
        .insert(*foot_soldier.get_id(), Box::new(foot_soldier));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors =
        state.get_interceptors_for_move(&path, rimland_nomads.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_stealthed_units_cannot_be_intercepted() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    rimland_nomads
        .get_unit_base_mut()
        .unwrap()
        .abilities
        .push(Ability::Stealth);
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state
        .cards
        .insert(*foot_soldier.get_id(), Box::new(foot_soldier));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors =
        state.get_interceptors_for_move(&path, rimland_nomads.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_voidwalking_interceptor_must_be_at_final_location() {
    let mut state = State::new_mock_state(vec![8, 13, 18]);
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state
        .cards
        .insert(*rimland_nomads.get_id(), Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = HeadlessHaunt::new(opponent_id);
    headless_haunt.set_zone(Zone::Location(Location::Square(12, Region::Surface)));
    state
        .cards
        .insert(*headless_haunt.get_id(), Box::new(headless_haunt.clone()));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors =
        state.get_interceptors_for_move(&path, rimland_nomads.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_airborne_unit_can_only_be_intercepted_by_airborne_or_ranged_units() {
    let mut state = State::new_mock_state(Vec::from_iter(1..=20));
    let player_id = state.players[0].id;
    let mut nimbus_jinn = NimbusJinn::new(player_id);
    nimbus_jinn.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state
        .cards
        .insert(*nimbus_jinn.get_id(), Box::new(nimbus_jinn.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state
        .cards
        .insert(*foot_soldier.get_id(), Box::new(foot_soldier.clone()));

    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state
        .cards
        .insert(*kite_archer.get_id(), Box::new(kite_archer.clone()));

    let path = vec![
        Zone::Location(Location::Square(8, Region::Surface)),
        Zone::Location(Location::Square(13, Region::Surface)),
        Zone::Location(Location::Square(18, Region::Surface)),
    ];
    let interceptors = state.get_interceptors_for_move(&path, nimbus_jinn.get_id(), &opponent_id);
    assert_eq!(interceptors.len(), 1);
    assert_eq!(&interceptors[0], kite_archer.get_id());
}

#[tokio::test]
async fn test_get_effective_costs_donnybrook_inn() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;
    let mut donnybrook_inn = DonnybrookInn::new(player_id);
    donnybrook_inn.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .cards
        .insert(*donnybrook_inn.get_id(), Box::new(donnybrook_inn.clone()));

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state
        .cards
        .insert(*cauldron_crones.get_id(), Box::new(cauldron_crones.clone()));

    state.reconcile_ongoing_effects_for_test().await.unwrap();
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

    state.reconcile_ongoing_effects_for_test().await.unwrap();
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

    let source = Zone::Location(Location::Square(8, Region::Surface));

    assert!(
        source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(Location::Square(9, Region::Surface)))
    );
    assert!(
        !source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(Location::Square(13, Region::Surface)))
    );
    assert!(
        source
            .get_adjacent_voids(&state)
            .contains(&Zone::Location(Location::Square(13, Region::Void)))
    );
    assert!(
        !source
            .get_adjacent_voids(&state)
            .contains(&Zone::Location(Location::Square(9, Region::Void)))
    );
}

#[test]
fn test_zone_query_adjacent_to_uses_state_aware_locations() {
    let state = State::new_mock_state(vec![8, 9]);
    let source = Zone::Location(Location::Square(13, Region::Void));

    let options = ZoneQuery::new().adjacent_to(&source).options(&state);

    assert!(options.contains(&Zone::Location(Location::Square(13, Region::Void))));
    assert!(!options.contains(&Zone::Location(Location::Square(8, Region::Surface))));
}

#[test]
fn test_card_query_adjacent_to_uses_state_aware_locations() {
    let mut state = State::new_mock_state(vec![8, 9]);
    let player_id = state.players[0].id;
    let source = Zone::Location(Location::Square(8, Region::Surface));

    let mut foot_soldier = FootSoldier::new(player_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
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

    let source = Zone::Location(Location::Square(13, Region::Void));

    assert!(
        !source
            .get_adjacent_locations(&state)
            .contains(&Zone::Location(Location::Square(8, Region::Surface)))
    );
    assert!(
        source
            .get_adjacent_sites(&state)
            .contains(&Zone::Location(Location::Square(8, Region::Surface)))
    );
}

#[test]
fn test_card_query_spatial_filters_resolve_with_current_state() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let source = Zone::Location(Location::Square(13, Region::Void));
    let query = CardQuery::new().sites().adjacent_sites_to(&source);

    let mut site = AridDesert::new(player_id);
    site.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    let site_id = *site.get_id();
    state.cards.insert(site_id, Box::new(site));

    assert!(query.matches(&site_id, &state));
}

#[tokio::test]
async fn test_sisters_of_silence_use_timestamp_order_for_dependent_effects() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    let mut older_sisters = SistersOfSilence::new(player_id);
    older_sisters.add_ability(Ability::Airborne);
    let older_id = insert_realm_card(
        &mut state,
        Box::new(older_sisters),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let mut newer_sisters = SistersOfSilence::new(player_id);
    newer_sisters.add_ability(Ability::Airborne);
    let newer_id = insert_realm_card(
        &mut state,
        Box::new(newer_sisters),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    assert!(
        state
            .get_card(&older_id)
            .has_ability(&state, &Ability::Airborne)
    );
    assert!(
        !state
            .get_card(&newer_id)
            .has_ability(&state, &Ability::Airborne)
    );
}

#[tokio::test]
async fn test_silence_removes_keyword_abilities_without_enumerating_values() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    insert_realm_card(
        &mut state,
        Box::new(SistersOfSilence::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let mut target = FootSoldier::new(player_id);
    target.add_ability(Ability::Movement(99));
    target.add_ability(Ability::Ranged(99));
    target.add_ability(Ability::CarryMinions(99));
    target.add_status(CardStatus::Disabled);
    let target_id = insert_realm_card(
        &mut state,
        Box::new(target),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    let target = state.get_card(&target_id);
    assert!(!target.has_ability(&state, &Ability::Movement(99)));
    assert!(!target.has_ability(&state, &Ability::Ranged(99)));
    assert!(!target.has_ability(&state, &Ability::CarryMinions(99)));
    assert!(target.has_status(&state, &CardStatus::Disabled));
}

#[tokio::test]
async fn test_silence_removes_special_negative_abilities_but_keeps_engine_statuses() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    insert_realm_card(
        &mut state,
        Box::new(SistersOfSilence::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let mut target = FootSoldier::new(player_id);
    target.add_ability(Ability::Immobile);
    target.add_ability(Ability::CannotDefend);
    target.add_status(CardStatus::Disabled);
    target.add_status(CardStatus::SummoningSickness);
    let target_id = insert_realm_card(
        &mut state,
        Box::new(target),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    let target = state.get_card(&target_id);
    assert!(!target.has_ability(&state, &Ability::Immobile));
    assert!(!target.has_ability(&state, &Ability::CannotDefend));
    assert!(target.has_status(&state, &CardStatus::Silenced));
    assert!(target.has_status(&state, &CardStatus::Disabled));
    assert!(target.has_status(&state, &CardStatus::SummoningSickness));
}

#[tokio::test]
async fn test_loses_all_abilities_keeps_disabled_status() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    let mut target = FootSoldier::new(player_id);
    target.add_ability(Ability::Airborne);
    target.add_ability(Ability::Immobile);
    target.add_status(CardStatus::Disabled);
    let target_id = insert_realm_card(
        &mut state,
        Box::new(target),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    state.ongoing_effects.push(TimedOngoingEffect {
        effect: ContinuousEffect::RemoveAbilities {
            removal: AbilityRemoval::AllAbilities,
            affected_cards: CardQuery::from_id(target_id),
        },
        source: None,
        timestamp: 1,
    });

    let target = state.get_card(&target_id);
    assert!(!target.has_ability(&state, &Ability::Airborne));
    assert!(!target.has_ability(&state, &Ability::Immobile));
    assert!(target.has_status(&state, &CardStatus::Disabled));
}

#[test]
fn test_game_is_over_only_when_one_player_remains() {
    let mut state = State::new_mock_state(vec![]);
    let first_player = state.players[0].clone();
    let second_player = state.players[1].clone();
    let third_player = Player {
        id: uuid::Uuid::new_v4(),
        name: "Player 3".to_string(),
    };
    state.players.push(third_player.clone());

    assert_eq!(state.living_players().len(), 3);
    assert!(state.winner_if_game_over().is_none());

    state.eliminate_player(first_player.id);
    assert_eq!(state.living_players().len(), 2);
    assert!(state.winner_if_game_over().is_none());

    state.eliminate_player(second_player.id);
    assert_eq!(
        state.winner_if_game_over().map(|player| player.id),
        Some(third_player.id)
    );
}

#[tokio::test]
async fn test_silence_removes_special_activated_abilities_but_keeps_basic_actions() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    insert_realm_card(
        &mut state,
        Box::new(SistersOfSilence::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    let target_id = insert_realm_card(
        &mut state,
        Box::new(SneakThief::new(player_id)),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    let action_names = state
        .get_card(&target_id)
        .get_activated_abilities(&state)
        .unwrap()
        .into_iter()
        .map(|action| action.get_name())
        .collect::<Vec<_>>();

    assert!(action_names.contains(&"Attack".to_string()));
    assert!(action_names.contains(&"Move".to_string()));
    assert!(!action_names.contains(&"Steal Artifact".to_string()));
}

#[tokio::test]
async fn test_exact_ability_removal_only_removes_that_ability() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    insert_realm_card(
        &mut state,
        Box::new(SkyBaron::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let mut target = FootSoldier::new(player_id);
    target.add_ability(Ability::Airborne);
    target.add_ability(Ability::Stealth);
    let target_id = insert_realm_card(
        &mut state,
        Box::new(target),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    let target = state.get_card(&target_id);
    assert!(!target.has_ability(&state, &Ability::Airborne));
    assert!(target.has_ability(&state, &Ability::Stealth));
}

#[tokio::test]
async fn test_smokestacks_of_gnaak_use_timestamp_order_for_dependent_effects() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    let older_id = insert_realm_card(
        &mut state,
        Box::new(SmokestacksOfGnaak::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    let newer_id = insert_realm_card(
        &mut state,
        Box::new(SmokestacksOfGnaak::new(player_id)),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;

    assert!(
        !state
            .get_card(&older_id)
            .has_status(&state, &CardStatus::Disabled)
    );
    assert!(
        state
            .get_card(&newer_id)
            .has_status(&state, &CardStatus::Disabled)
    );
}

#[tokio::test]
async fn test_flood_and_drought_use_timestamp_order_for_water_affinity() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let site_id = insert_realm_card(
        &mut state,
        Box::new(AridDesert::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    insert_realm_card(
        &mut state,
        Box::new(Flood::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    insert_realm_card(
        &mut state,
        Box::new(Drought::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let site = state.get_card(&site_id).get_resource_provider().unwrap();
    assert_eq!(site.provided_affinity(&state).unwrap().water, 0);

    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let site_id = insert_realm_card(
        &mut state,
        Box::new(AridDesert::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    insert_realm_card(
        &mut state,
        Box::new(Drought::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    insert_realm_card(
        &mut state,
        Box::new(Flood::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    let site = state.get_card(&site_id).get_resource_provider().unwrap();
    assert_eq!(site.provided_affinity(&state).unwrap().water, 1);
}

#[tokio::test]
async fn test_flood_grants_flooded_ability() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let site_id = insert_realm_card(
        &mut state,
        Box::new(AridDesert::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    state.ongoing_effects.push(TimedOngoingEffect {
        effect: ContinuousEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: CardQuery::from_id(site_id),
        },
        source: None,
        timestamp: 1,
    });
    state.invalidate_runtime_caches();

    let site = state.get_card(&site_id);
    assert!(site.has_ability(&state, &Ability::Flooded));
    assert_eq!(
        site.get_resource_provider()
            .unwrap()
            .provided_affinity(&state)
            .unwrap()
            .water,
        1
    );
}

#[tokio::test]
async fn test_removing_abilities_removes_flooded() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let site_id = insert_realm_card(
        &mut state,
        Box::new(AridDesert::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;

    state.ongoing_effects.push(TimedOngoingEffect {
        effect: ContinuousEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: CardQuery::from_id(site_id),
        },
        source: None,
        timestamp: 1,
    });
    state.ongoing_effects.push(TimedOngoingEffect {
        effect: ContinuousEffect::RemoveAbilities {
            removal: AbilityRemoval::AllAbilities,
            affected_cards: CardQuery::from_id(site_id),
        },
        source: None,
        timestamp: 2,
    });
    state.invalidate_runtime_caches();

    let site = state.get_card(&site_id);
    assert!(!site.has_ability(&state, &Ability::Flooded));
    assert_eq!(
        site.get_resource_provider()
            .unwrap()
            .provided_affinity(&state)
            .unwrap()
            .water,
        0
    );
}

#[tokio::test]
async fn test_passive_ongoing_effect_lifecycle_tracks_realm_entry_and_exit() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let source_id = insert_realm_card(
        &mut state,
        Box::new(DonnybrookInn::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    let first_timestamps = passive_ongoing_timestamps_for_source(&state, &source_id);
    assert!(!first_timestamps.is_empty());

    Effect::SetCardZone {
        card_id: source_id,
        zone: Zone::Location(Location::Square(8, Region::Surface)),
    }
    .apply(&mut state)
    .await
    .unwrap();
    let moved_timestamps = passive_ongoing_timestamps_for_source(&state, &source_id);
    assert_eq!(moved_timestamps, first_timestamps);
    state.reconcile_ongoing_effects_for_test().await.unwrap();
    assert_eq!(
        passive_ongoing_timestamps_for_source(&state, &source_id),
        first_timestamps
    );

    Effect::SetCardZone {
        card_id: source_id,
        zone: Zone::Hand,
    }
    .apply(&mut state)
    .await
    .unwrap();
    assert!(passive_ongoing_timestamps_for_source(&state, &source_id).is_empty());

    Effect::SetCardZone {
        card_id: source_id,
        zone: Zone::Location(Location::Square(9, Region::Surface)),
    }
    .apply(&mut state)
    .await
    .unwrap();
    let reentered_timestamps = passive_ongoing_timestamps_for_source(&state, &source_id);
    assert!(!reentered_timestamps.is_empty());
    assert!(reentered_timestamps.iter().all(|timestamp| {
        first_timestamps
            .iter()
            .all(|first_timestamp| timestamp > first_timestamp)
    }));
}

#[tokio::test]
async fn test_source_relative_ongoing_effects_follow_source_without_refreshing() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;

    let source_id = insert_realm_card(
        &mut state,
        Box::new(SmokestacksOfGnaak::new(player_id)),
        Zone::Location(Location::Square(7, Region::Surface)),
    )
    .await;
    let target_id = insert_realm_card(
        &mut state,
        Box::new(SmokestacksOfGnaak::new(player_id)),
        Zone::Location(Location::Square(8, Region::Surface)),
    )
    .await;
    let initial_timestamps = passive_ongoing_timestamps_for_source(&state, &source_id);

    assert!(
        state
            .get_card(&target_id)
            .has_status(&state, &CardStatus::Disabled)
    );

    let target_zone = Zone::Location(Location::Square(8, Region::Surface));
    let new_source_zone = Zone::all_realm()
        .into_iter()
        .find(|zone| zone != &target_zone && !zone.get_nearby_sites(&state).contains(&target_zone))
        .expect("a non-nearby source zone should exist");
    Effect::SetCardZone {
        card_id: source_id,
        zone: new_source_zone,
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert_eq!(
        passive_ongoing_timestamps_for_source(&state, &source_id),
        initial_timestamps
    );
    assert!(
        !state
            .get_card(&target_id)
            .has_status(&state, &CardStatus::Disabled)
    );
}

#[test]
fn test_rubble_has_no_controller() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut rubble = Rubble::new(player_id);
    let rubble_id = *rubble.get_id();
    rubble.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    state.cards.insert(rubble_id, Box::new(rubble));

    assert_eq!(
        state.get_card(&rubble_id).get_controller_id(&state),
        NO_CONTROLLER
    );
    assert!(
        !CardQuery::new()
            .sites()
            .controlled_by(&player_id)
            .all(&state)
            .contains(&rubble_id)
    );
    assert!(
        !CardQuery::new()
            .sites()
            .controlled_by(&opponent_id)
            .all(&state)
            .contains(&rubble_id)
    );
    assert_eq!(
        state.get_thresholds_for_player(&player_id),
        Thresholds::ZERO
    );
    assert_eq!(
        state.get_thresholds_for_player(&opponent_id),
        Thresholds::ZERO
    );
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

#[test]
fn test_turn_iterator_skips_next_turn() {
    let player_one = uuid::Uuid::new_v4();
    let player_two = uuid::Uuid::new_v4();
    let player_three = uuid::Uuid::new_v4();
    let mut it = TurnIterator::new(vec![player_one, player_two, player_three]);

    it.skip_next_for(&player_two);

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_three);
}

#[test]
fn test_turn_iterator_skips_future_multiplayer_turn() {
    let player_one = uuid::Uuid::new_v4();
    let player_two = uuid::Uuid::new_v4();
    let player_three = uuid::Uuid::new_v4();
    let mut it = TurnIterator::new(vec![player_one, player_two, player_three]);

    it.skip_next_for(&player_three);

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_two);

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_one);
}

#[test]
fn test_turn_iterator_skips_overridden_turn() {
    let player_one = uuid::Uuid::new_v4();
    let player_two = uuid::Uuid::new_v4();
    let player_three = uuid::Uuid::new_v4();
    let mut it = TurnIterator::new(vec![player_one, player_two, player_three]);

    it.override_next(Turn::controlled_by(player_three, player_one));
    it.skip_next_for(&player_three);

    let curr = it.next();
    assert!(curr.is_some());
    assert_eq!(curr.unwrap().player_id, player_two);
}

#[tokio::test]
async fn test_skip_next_turn_effect_updates_turn_order() {
    let (mut state, _rx) = setup_carrying_state();
    let player_one = state.players[0].id;
    let player_two = state.players[1].id;

    state.queue_one(Effect::SkipNextTurn {
        player_id: player_two,
    });
    state.apply_effects_without_log().await.unwrap();

    assert_eq!(state.next_turn().player_id(), player_one);
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
