use crate::{
    card::{
        Ability, AridDesert, BeastOfBurden, Card, CauldronCrones, DonnybrookInn, Enchantress,
        FootSoldier, HeadlessHaunt, KiteArcher, NimbusJinn, RimlandNomads, Zone,
        from_name_and_zone,
    },
    deck::Deck,
    effect::Effect,
    game::Thresholds,
    networking::message::ServerMessage,
    query::{EffectQuery, QueryCache, ZoneQuery},
    state::{CardQuery, Player, PlayerWithDeck, State, TemporaryEffect},
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
    carrier.set_zone(Zone::Realm(1));
    state.cards.push(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Realm(1));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.push(Box::new(passenger));

    // Move carrier to Realm(2)
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: carrier_id,
        from: Zone::Realm(1),
        to: ZoneQuery::from_zone(Zone::Realm(2)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(state.get_card(&carrier_id).get_zone(), &Zone::Realm(2));
    assert_eq!(state.get_card(&passenger_id).get_zone(), &Zone::Realm(2));
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
    carrier.set_zone(Zone::Realm(1));
    state.cards.push(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Realm(1));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.push(Box::new(passenger));

    // Move passenger to Realm(2) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Realm(1),
        to: ZoneQuery::from_zone(Zone::Realm(2)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: None,
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(state.get_card(&carrier_id).get_zone(), &Zone::Realm(1));
    assert_eq!(state.get_card(&passenger_id).get_zone(), &Zone::Realm(2));
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
    carrier.set_zone(Zone::Realm(1));
    state.cards.push(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Realm(1));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.push(Box::new(passenger));

    // Move passenger through Realm(2) to Realm(3) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Realm(1),
        to: ZoneQuery::from_zone(Zone::Realm(3)),
        tap: false,
        region: crate::card::Region::Surface,
        through_path: Some(vec![Zone::Realm(1), Zone::Realm(2), Zone::Realm(3)]),
    };

    move_effect.apply(&mut state).await.unwrap();

    assert_eq!(state.get_card(&carrier_id).get_zone(), &Zone::Realm(1));
    assert_eq!(state.get_card(&passenger_id).get_zone(), &Zone::Realm(3));
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
    carrier.set_zone(Zone::Realm(1));
    carrier.add_ability(Ability::Airborne);
    carrier.add_ability(Ability::Voidwalk);
    state.cards.push(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Realm(1));
    passenger.set_bearer_id(Some(carrier_id));
    state.cards.push(Box::new(passenger));

    let passenger_abilities = state.get_card(&passenger_id).get_abilities(&state).unwrap();
    assert!(passenger_abilities.contains(&Ability::Airborne));
    assert!(passenger_abilities.contains(&Ability::Voidwalk));
    assert!(!passenger_abilities.contains(&Ability::Burrowing));
    assert!(!passenger_abilities.contains(&Ability::Submerge));

    // Move passenger away, abilities should be lost
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Zone::Realm(1),
        to: ZoneQuery::from_zone(Zone::Realm(2)),
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
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Realm(12));
    state.cards.push(Box::new(kite_archer.clone()));

    let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 1);
    assert_eq!(&interceptors[0].0, kite_archer.get_id());
}

#[test]
fn test_no_inteceptors() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Realm(11));
    state.cards.push(Box::new(kite_archer.clone()));

    let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 0);
}

#[test]
fn test_voidwalking_interceptor() {
    let mut state = State::new_mock_state(vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)]);
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = HeadlessHaunt::new(opponent_id);
    headless_haunt.set_zone(Zone::Realm(12));
    state.cards.push(Box::new(headless_haunt.clone()));

    let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 1);
}

#[test]
fn test_airborne_interceptor() {
    let mut state = State::new_mock_state(Zone::all_realm());
    let player_id = state.players[0].id;
    let mut rimland_nomads = RimlandNomads::new(player_id);
    rimland_nomads.set_zone(Zone::Realm(8));
    state.cards.push(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = NimbusJinn::new(opponent_id);
    headless_haunt.set_zone(Zone::Realm(12));
    state.cards.push(Box::new(headless_haunt.clone()));

    let path = vec![Zone::Realm(8), Zone::Realm(13), Zone::Realm(18)];
    let interceptors = state.get_interceptors_for_move(&path, &opponent_id);
    assert_eq!(interceptors.len(), 3);
}

#[tokio::test]
async fn test_get_effective_costs_donnybrook_inn() {
    let mut state = State::new_mock_state(vec![Zone::Realm(8)]);
    let player_id = state.players[0].id;
    let mut donnybrook_inn = DonnybrookInn::new(player_id);
    donnybrook_inn.set_zone(Zone::Realm(3));
    state.cards.push(Box::new(donnybrook_inn.clone()));

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state.cards.push(Box::new(cauldron_crones.clone()));

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
    let mut state = State::new_mock_state(vec![Zone::Realm(8)]);
    let player_id = state.players[0].id;

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state.cards.push(Box::new(cauldron_crones.clone()));

    state.compute_world_effects().await.unwrap();
    let regular_costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(regular_costs.mana_value(), 3);
    assert_eq!(regular_costs.thresholds_cost(), &Thresholds::parse("F"));

    state
        .temporary_effects
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
