use crate::{
    card::{
        Ability, AridDesert, BeastOfBurden, BlastedOak, Card, CardStatus, CauldronCrones,
        CourtJester, CourtesanThais, DonnybrookInn, Drought, Enchantress, Flood, FootSoldier,
        FreeCity, HeadlessHaunt, KiteArcher, KytheraMechanism, LuckyCharm, NimbusJinn, Region,
        RimlandNomads, Rubble, Silence, SistersOfSilence, SkyBaron, SmokestacksOfGnaak,
        SneakThief, UnitBase, from_name_and_zone,
    },
    deck::Deck,
    effect::{Effect, FightContext},
    game::{Direction, NO_CONTROLLER, Thresholds},
    networking::message::{ClientMessage, ServerMessage},
    query::{CardQuery, EffectQuery, LocationQuery, QueryCache, ZoneQuery},
    state::{
        AbilityRemoval, OngoingEffect, Player, PlayerWithDeck, State, TemporaryEffect,
        TimedOngoingEffect, Turn, TurnIterator,
    },
    zone::{Location, Zone},
};

fn setup_carrying_state() -> (State, async_channel::Receiver<ServerMessage>) {
    let (state, server_rx, _) = setup_carrying_state_with_client();
    (state, server_rx)
}

fn setup_carrying_state_with_client() -> (
    State,
    async_channel::Receiver<ServerMessage>,
    async_channel::Sender<ClientMessage>,
) {
    QueryCache::init();

    let player_one_id = uuid::Uuid::new_v4();
    let player_two_id = uuid::Uuid::new_v4();

    let avatar_one = Enchantress::new(player_one_id);
    let avatar_one_id = *avatar_one.get_id();
    let avatar_two = Enchantress::new(player_two_id);
    let avatar_two_id = *avatar_two.get_id();

    let locations = Location::all_in_region(Region::Surface);
    let mut p1_cards: Vec<Box<dyn Card>> = locations
        .iter()
        .map(|l| from_name_and_zone(AridDesert::NAME, &player_one_id, l.clone().into()))
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
    let (client_tx, client_rx) = async_channel::unbounded();
    let state = State::new(
        uuid::Uuid::new_v4(),
        vec![player1, player2],
        server_tx,
        client_rx,
    );
    (state, server_rx, client_tx)
}

async fn insert_realm_card(state: &mut State, mut card: Box<dyn Card>, zone: Zone) -> uuid::Uuid {
    let card_id = *card.get_id();
    card.set_zone(zone);
    state.add_card(card);
    state
        .add_passive_ongoing_effects_for_source(&card_id)
        .await
        .unwrap();
    card_id
}

#[tokio::test]
async fn test_location_direction_wraps_when_edges_are_connected() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.player_one;
    let unit_id = insert_realm_card(
        &mut state,
        Box::new(FootSoldier::new(player_id)),
        Zone::Location(Location::Square(3, Region::Surface)),
    )
    .await;

    state.ongoing_effects.push(TimedOngoingEffect {
        effect: OngoingEffect::ConnectTopBottomEdges {
            affected_cards: Box::new(CardQuery::new().in_play()),
        },
        source: None,
        timestamp: 1,
    });

    let location = Location::Square(3, Region::Surface);
    assert_eq!(
        location.step_in_direction(&Direction::Down, &state, Some(&unit_id)),
        Some(Location::Square(18, Region::Surface))
    );
    assert_eq!(
        location.step_in_direction(&Direction::Down, &state, None),
        None
    );
}

#[tokio::test]
async fn test_lucky_charm_narrows_random_card_query_through_ongoing_effect() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let game_id = state.game_id;

    insert_realm_card(
        &mut state,
        Box::new(LuckyCharm::new(player_id)),
        Zone::Location(Location::Square(1, Region::Surface)),
    )
    .await;

    let targets = vec![
        insert_realm_card(
            &mut state,
            Box::new(FootSoldier::new(player_id)),
            Zone::Location(Location::Square(2, Region::Surface)),
        )
        .await,
        insert_realm_card(
            &mut state,
            Box::new(FootSoldier::new(player_id)),
            Zone::Location(Location::Square(3, Region::Surface)),
        )
        .await,
        insert_realm_card(
            &mut state,
            Box::new(FootSoldier::new(player_id)),
            Zone::Location(Location::Square(4, Region::Surface)),
        )
        .await,
    ];

    let query = CardQuery::from_ids(targets.clone()).randomised();
    let pick = query.pick(&player_id, &state);
    tokio::pin!(pick);

    let mut saw_choice = false;
    let picked = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick => break result.unwrap().unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::Wait { .. } | ServerMessage::Resume { .. } => {}
                    ServerMessage::PickCard { cards, .. } => {
                        assert_eq!(cards.len(), 2);
                        assert!(cards.iter().all(|id| targets.contains(id)));
                        let card_id = cards[0];
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id,
                            })
                            .await
                            .unwrap();
                        saw_choice = true;
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();

    assert!(saw_choice);
    assert!(targets.contains(&picked));
}

#[tokio::test]
async fn test_kythera_mechanism_converts_random_queries_to_choices() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let game_id = state.game_id;

    let bearer_id = insert_realm_card(
        &mut state,
        Box::new(FootSoldier::new(player_id)),
        Zone::Location(Location::Square(1, Region::Surface)),
    )
    .await;
    let mut kythera = KytheraMechanism::new(player_id);
    kythera.set_bearer_id(Some(bearer_id));
    insert_realm_card(
        &mut state,
        Box::new(kythera),
        Zone::Location(Location::Square(1, Region::Surface)),
    )
    .await;

    let targets = vec![
        insert_realm_card(
            &mut state,
            Box::new(FootSoldier::new(player_id)),
            Zone::Location(Location::Square(2, Region::Surface)),
        )
        .await,
        insert_realm_card(
            &mut state,
            Box::new(FootSoldier::new(player_id)),
            Zone::Location(Location::Square(3, Region::Surface)),
        )
        .await,
    ];

    let card_query = CardQuery::from_ids(targets.clone()).randomised();
    let pick_card = card_query.pick(&player_id, &state);
    tokio::pin!(pick_card);

    let picked_card = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick_card => break result.unwrap().unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::Wait { .. } | ServerMessage::Resume { .. } => {}
                    ServerMessage::PickCard { cards, .. } => {
                        assert_eq!(cards.len(), targets.len());
                        assert!(cards.iter().all(|id| targets.contains(id)));
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: cards[0],
                            })
                            .await
                            .unwrap();
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();
    assert!(targets.contains(&picked_card));

    let zones = vec![
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
    ];
    let zone_query = ZoneQuery::random(zones.clone());
    let pick_zone = zone_query.pick(&player_id, &state);
    tokio::pin!(pick_zone);

    let picked_zone = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick_zone => break result.unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::Resume { .. } => {}
                    ServerMessage::PickLocation { locations: offered, .. } => {
                        assert_eq!(offered.len(), zones.len());
                        assert!(offered.iter().all(|loc| zones.contains(&loc.into())));
                        client_tx
                            .send(ClientMessage::PickLocation {
                                game_id,
                                player_id,
                                location: offered[0].clone(),
                            })
                            .await
                            .unwrap();
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();
    assert!(zones.contains(&picked_zone));
}

#[tokio::test]
async fn test_kythera_mechanism_lets_bearer_controller_choose_court_jester_discard() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let game_id = state.game_id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    let opponent_avatar_id = state.get_player_avatar_id(&opponent_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state
        .get_card_mut(&opponent_avatar_id)
        .set_zone(Zone::Location(Location::Square(3, Region::Surface)));

    let mut first_card = FootSoldier::new(opponent_id);
    let first_card_id = *first_card.get_id();
    first_card.set_zone(Zone::Hand);
    state.add_card(Box::new(first_card));

    let mut chosen_card = FootSoldier::new(opponent_id);
    let chosen_card_id = *chosen_card.get_id();
    chosen_card.set_zone(Zone::Hand);
    state.add_card(Box::new(chosen_card));

    let mut jester = CourtJester::new(player_id);
    jester.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(jester.clone()));

    let mut kythera = KytheraMechanism::new(player_id);
    let kythera_id = *kythera.get_id();
    kythera.set_zone(Zone::Hand);
    state.add_card(Box::new(kythera));
    state.queue(vec![
        Effect::SetBearer {
            card_id: kythera_id,
            bearer_id: Some(avatar_id),
        },
        Effect::PlayCard {
            player_id,
            card_id: kythera_id,
            location: Location::Square(1, Region::Surface),
            spellcaster: avatar_id,
        },
    ]);
    state.apply_effects_without_log().await.unwrap();

    let expected_decision_player_id = player_id;
    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            if let ServerMessage::PickCard {
                player_id, cards, ..
            } = message
            {
                assert_eq!(player_id, expected_decision_player_id);
                assert!(cards.contains(&first_card_id));
                assert!(cards.contains(&chosen_card_id));
                client_tx
                    .send(ClientMessage::PickCard {
                        game_id,
                        player_id,
                        card_id: chosen_card_id,
                    })
                    .await
                    .unwrap();
            }
        }
    });

    let effects = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        jester.resolve_hook(1, &state, &Effect::EndTurn { player_id }),
    )
    .await
    .unwrap()
    .unwrap();

    assert!(matches!(
        effects.as_slice(),
        [Effect::DiscardCard { card_id, .. }] if *card_id == chosen_card_id
    ));
    assert_eq!(state.get_card(&first_card_id).get_zone(), &Zone::Hand);
    assert_eq!(state.get_card(&chosen_card_id).get_zone(), &Zone::Hand);
}

#[tokio::test]
async fn test_kythera_mechanism_casting_prompts_for_unit_or_site() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let game_id = state.game_id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(2, Region::Surface)));

    let mut kythera = KytheraMechanism::new(player_id);
    let kythera_id = *kythera.get_id();
    kythera.set_zone(Zone::Hand);
    state.add_card(Box::new(kythera.clone()));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickAction {
                    player_id,
                    actions,
                    ..
                } => {
                    assert_eq!(actions, vec!["Equip to unit", "Play atop site"]);
                    client_tx
                        .send(ClientMessage::PickAction {
                            game_id,
                            player_id,
                            action_idx: 0,
                        })
                        .await
                        .unwrap();
                }
                ServerMessage::PickCard {
                    player_id, cards, ..
                } => {
                    assert!(!cards.is_empty());
                    client_tx
                        .send(ClientMessage::PickCard {
                            game_id,
                            player_id,
                            card_id: cards[0],
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    let effects = kythera
        .play_mechanic(&state, &player_id, &avatar_id)
        .await
        .unwrap();

    assert_eq!(effects.len(), 2);
    assert!(matches!(
        effects.as_slice(),
        [Effect::SetBearer {
            card_id,
            bearer_id: Some(_),
        }, Effect::PlayCard {
            card_id: played_card_id,
            ..
        }] if *card_id == kythera_id && *played_card_id == kythera_id
    ));
}

#[tokio::test]
async fn test_blasted_oak_restricts_card_targets_by_precedence() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let game_id = state.game_id;
    let oak_location = Location::Square(1, Region::Surface);

    let oak_id = insert_realm_card(
        &mut state,
        Box::new(BlastedOak::new(player_id)),
        Zone::Location(oak_location.clone()),
    )
    .await;
    let minion_id = insert_realm_card(
        &mut state,
        Box::new(FootSoldier::new(player_id)),
        Zone::Location(oak_location.clone()),
    )
    .await;
    let source = Drought::new(player_id);
    let source_id = *source.get_id();
    state.add_card(Box::new(source));
    let site_id = CardQuery::new()
        .sites()
        .in_location(oak_location)
        .first(&state)
        .expect("failed to find site");

    let targets = vec![site_id, minion_id, oak_id];
    let query = CardQuery::from_ids(targets).with_source_card(source_id);
    let pick = query.pick(&player_id, &state);
    tokio::pin!(pick);

    let picked = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick => break result.unwrap().unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::Wait { .. } | ServerMessage::Resume { .. } => {}
                    ServerMessage::PickCard { cards, .. } => {
                        assert_eq!(cards, vec![oak_id]);
                        client_tx
                            .send(ClientMessage::PickCard {
                                game_id,
                                player_id,
                                card_id: cards[0],
                            })
                            .await
                            .unwrap();
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(picked, oak_id);
}

#[tokio::test]
async fn test_blasted_oak_restricts_zone_targets_only_with_source() {
    let (mut state, server_rx, client_tx) = setup_carrying_state_with_client();
    let player_id = state.players[0].id;
    let game_id = state.game_id;
    let oak_zone = Location::Square(1, Region::Surface);
    let other_zone = Location::Square(2, Region::Surface);

    insert_realm_card(
        &mut state,
        Box::new(BlastedOak::new(player_id)),
        oak_zone.clone().into(),
    )
    .await;
    let source = Drought::new(player_id);
    let source_id = *source.get_id();
    state.add_card(Box::new(source));

    let query = ZoneQuery::from_options(
        vec![oak_zone.clone().into(), other_zone.clone().into()],
        Some("Pick a location".to_string()),
    )
    .with_source_card(source_id);
    let pick = query.pick(&player_id, &state);
    tokio::pin!(pick);

    let picked = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick => break result.unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::PickLocation { locations, .. } => {
                        assert_eq!(locations, vec![oak_zone.clone()]);
                        client_tx
                            .send(ClientMessage::PickLocation {
                                game_id,
                                player_id,
                                location: locations[0].clone(),
                            })
                            .await
                            .unwrap();
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(picked, oak_zone.clone().into());

    let query_without_source = ZoneQuery::from_options(
        vec![oak_zone.clone().into(), other_zone.clone().into()],
        Some("Pick a location".to_string()),
    );
    let pick_without_source = query_without_source.pick(&player_id, &state);
    tokio::pin!(pick_without_source);

    let picked_without_source = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            tokio::select! {
                result = &mut pick_without_source => break result.unwrap(),
                msg = server_rx.recv() => match msg.unwrap() {
                    ServerMessage::PickLocation { locations, .. } => {
                        assert_eq!(locations, vec![oak_zone.clone(), other_zone.clone()]);
                        client_tx
                            .send(ClientMessage::PickLocation {
                                game_id,
                                player_id,
                                location: other_zone.clone(),
                            })
                            .await
                            .unwrap();
                    }
                    other => panic!("unexpected server message: {:?}", other),
                },
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(picked_without_source, other_zone.into());
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

#[tokio::test]
async fn test_free_city_can_activate_attack_before_animation() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let zone = Zone::Location(Location::Square(1, Region::Surface));

    let free_city_id =
        insert_realm_card(&mut state, Box::new(FreeCity::new(player_id)), zone.clone()).await;
    insert_realm_card(&mut state, Box::new(FootSoldier::new(opponent_id)), zone).await;

    assert!(!state.is_unit_card(&free_city_id));
    let action_names = state
        .get_card(&free_city_id)
        .get_activated_abilities(&state)
        .unwrap()
        .into_iter()
        .filter(|action| {
            action
                .get_cost(&free_city_id, &state)
                .and_then(|cost| cost.can_afford(&state, player_id))
                .unwrap_or_default()
                && action
                    .can_activate(&free_city_id, &player_id, &state)
                    .unwrap_or_default()
        })
        .map(|action| action.get_name())
        .collect::<Vec<_>>();

    assert!(
        action_names.contains(&"Attack or defend against enemies here".to_string()),
        "Free City should expose its attack/defend ability when an enemy unit is here"
    );
}

#[tokio::test]
async fn test_free_city_animates_when_declared_as_defender() {
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
    assert!(
        state
            .get_defenders_for_attack(&attacker_id, &avatar_id)
            .contains(&free_city_id)
    );

    state.queue_one(Effect::DeclareDefender {
        attacker_id,
        defender_id: free_city_id,
    });
    state.apply_effects_without_log().await.unwrap();

    assert!(state.is_unit_card(&free_city_id));
    assert!(
        !state
            .get_defenders_for_attack(&attacker_id, &avatar_id)
            .contains(&free_city_id),
        "Free City should be marked used after defending"
    );

    state.queue_one(Effect::StartTurn { player_id });
    state.apply_effects_without_log().await.unwrap();

    assert!(!state.is_unit_card(&free_city_id));
    assert!(
        state
            .get_defenders_for_attack(&attacker_id, &avatar_id)
            .contains(&free_city_id),
        "Free City should reset on its controller's next turn start"
    );
}

#[tokio::test]
async fn test_animated_free_city_gets_unit_actions() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let zone = Zone::Location(Location::Square(1, Region::Surface));

    let free_city_id =
        insert_realm_card(&mut state, Box::new(FreeCity::new(player_id)), zone.clone()).await;
    insert_realm_card(&mut state, Box::new(FootSoldier::new(opponent_id)), zone).await;

    state.queue_one(Effect::Animate {
        card_id: free_city_id,
        unit_base: UnitBase {
            power: 3,
            toughness: 3,
            ..Default::default()
        },
        expires_on_effect: EffectQuery::TurnStart {
            player_id: Some(player_id),
        },
    });
    state.apply_effects_without_log().await.unwrap();

    let action_names = state
        .get_card(&free_city_id)
        .get_activated_abilities(&state)
        .unwrap()
        .into_iter()
        .map(|action| action.get_name())
        .collect::<Vec<_>>();

    assert!(action_names.contains(&"Attack".to_string()));
    assert!(action_names.contains(&"Move".to_string()));
    assert!(action_names.contains(&"Attack or defend against enemies here".to_string()));
}

#[tokio::test]
async fn test_free_city_defender_declaration_resolves_before_move_and_attack() {
    let (mut state, _rx) = setup_carrying_state();
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let zone = Zone::Location(Location::Square(1, Region::Surface));

    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state.get_card_mut(&avatar_id).set_zone(zone.clone());
    let free_city_id =
        insert_realm_card(&mut state, Box::new(FreeCity::new(player_id)), zone.clone()).await;
    let attacker_id = insert_realm_card(
        &mut state,
        Box::new(FootSoldier::new(opponent_id)),
        zone.clone(),
    )
    .await;

    state.queue(vec![
        Effect::Fight {
            attacker_id,
            defender_id: free_city_id,
            defending_ids: vec![],
            damage_assignment: None,
            context: FightContext::Attack,
        },
        Effect::MoveCard {
            player_id,
            card_id: free_city_id,
            from: zone
                .clone()
                .location()
                .cloned()
                .expect("test zone must be a location"),
            to: LocationQuery::from_zone(zone),
            tap: true,
            through_path: None,
        },
        Effect::DeclareDefender {
            attacker_id,
            defender_id: free_city_id,
        },
    ]);

    state.apply_effects_without_log().await.unwrap();

    assert!(state.is_unit_card(&free_city_id));
    assert_eq!(state.get_card(&attacker_id).get_zone(), &Zone::Cemetery);
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
    state.add_card(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

    // Move carrier to Realm(2)
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: carrier_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(crate::card::Region::Surface),
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
    state.add_card(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

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
    state.add_card(Box::new(carrier));

    let mut passenger = RimlandNomads::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

    state.queue_one(Effect::SetCardRegion {
        card_id: carrier_id,
        destination: Region::Underground,
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
    state.add_card(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

    // Move passenger to Realm(2) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(crate::card::Region::Surface),
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
    state.add_card(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

    // Move passenger through Realm(2) to Realm(3) independently
    let move_effect = Effect::MoveCard {
        player_id,
        card_id: passenger_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(3, Region::Surface).with_region(crate::card::Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Location::Square(1, Region::Surface),
            Location::Square(2, Region::Surface),
            Location::Square(3, Region::Surface),
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
    state.add_card(Box::new(carrier));

    let mut passenger = FootSoldier::new(player_id);
    let passenger_id = *passenger.get_id();
    passenger.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    passenger.set_bearer_id(Some(carrier_id));
    state.add_card(Box::new(passenger));

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
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(crate::card::Region::Surface),
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
    state.add_card(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state.add_card(Box::new(kite_archer.clone()));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    state.add_card(Box::new(kite_archer.clone()));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    foot_soldier.set_tapped(true);
    state.add_card(Box::new(foot_soldier));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state.add_card(Box::new(foot_soldier));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(rimland_nomads.clone()));

    let opponent_id = state.players[1].id;
    let mut headless_haunt = HeadlessHaunt::new(opponent_id);
    headless_haunt.set_zone(Zone::Location(Location::Square(12, Region::Surface)));
    state.add_card(Box::new(headless_haunt.clone()));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(nimbus_jinn.clone()));

    let opponent_id = state.players[1].id;
    let mut foot_soldier = FootSoldier::new(opponent_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state.add_card(Box::new(foot_soldier.clone()));

    let mut kite_archer = KiteArcher::new(opponent_id);
    kite_archer.set_zone(Zone::Location(Location::Square(18, Region::Surface)));
    state.add_card(Box::new(kite_archer.clone()));

    let path = vec![
        Location::Square(8, Region::Surface),
        Location::Square(13, Region::Surface),
        Location::Square(18, Region::Surface),
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
    state.add_card(Box::new(donnybrook_inn.clone()));

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state.add_card(Box::new(cauldron_crones.clone()));

    state.reconcile_ongoing_effects_for_test().await.unwrap();
    let regular_costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(regular_costs.printed_mana_value(), Some(3));
    assert_eq!(
        regular_costs
            .payable_options()
            .first()
            .and_then(|cost| cost.payable_mana_value()),
        Some(3)
    );

    let inn_costs = state
        .get_effective_costs(
            cauldron_crones.get_id(),
            Some(donnybrook_inn.get_location()),
            &player_id,
        )
        .unwrap();
    assert_eq!(inn_costs.printed_mana_value(), Some(3));
    assert_eq!(
        inn_costs
            .payable_options()
            .first()
            .and_then(|cost| cost.payable_mana_value()),
        Some(2)
    );
}

#[tokio::test]
async fn test_get_effective_costs_ignoring_thresholds() {
    let mut state = State::new_mock_state(vec![8]);
    let player_id = state.players[0].id;

    let mut cauldron_crones = CauldronCrones::new(player_id);
    cauldron_crones.set_zone(Zone::Hand);
    state.add_card(Box::new(cauldron_crones.clone()));

    state.reconcile_ongoing_effects_for_test().await.unwrap();
    let regular_costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(regular_costs.printed_mana_value(), Some(3));
    assert_eq!(regular_costs.printed_thresholds(), &Thresholds::parse("F"));
    assert_eq!(
        regular_costs
            .payable_options()
            .first()
            .map(|cost| cost.payable_thresholds()),
        Some(&Thresholds::parse("F"))
    );

    state
        .temporary_effects_mut()
        .push(TemporaryEffect::IgnoreCostThresholds {
            affected_cards: Box::new(
                std::convert::Into::<CardQuery>::into(cauldron_crones.get_id())
                    .including_not_in_play(),
            ),
            expires_on_effect: EffectQuery::TurnEnd { player_id: None },
            for_player: player_id,
        });
    let costs = state
        .get_effective_costs(cauldron_crones.get_id(), None, &player_id)
        .unwrap();
    assert_eq!(costs.printed_mana_value(), Some(3));
    assert_eq!(costs.printed_thresholds(), &Thresholds::parse("F"));
    assert_eq!(
        costs
            .payable_options()
            .first()
            .map(|cost| cost.payable_thresholds()),
        Some(&Thresholds::ZERO)
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
    let source = Location::Square(8, Region::Surface);

    let mut foot_soldier = FootSoldier::new(player_id);
    foot_soldier.set_zone(Zone::Location(Location::Square(13, Region::Surface)));
    let foot_soldier_id = *foot_soldier.get_id();
    state.add_card(Box::new(foot_soldier));

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

    let source = Location::Square(13, Region::Void);

    assert!(
        !source
            .get_adjacent(&state)
            .contains(&Location::Square(8, Region::Surface))
    );
    assert!(
        source
            .get_adjacent_sites(&state)
            .contains(&Location::Square(8, Region::Surface))
    );
}

#[test]
fn test_card_query_spatial_filters_resolve_with_current_state() {
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;
    let source = Location::Square(13, Region::Void);
    let query = CardQuery::new().sites().adjacent_sites_to(&source);

    let mut site = AridDesert::new(player_id);
    site.set_zone(Zone::Location(Location::Square(8, Region::Surface)));
    let site_id = *site.get_id();
    state.add_card(Box::new(site));

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
        effect: OngoingEffect::RemoveAbilities {
            removal: AbilityRemoval::AllAbilities,
            affected_cards: Box::new(CardQuery::from_id(target_id)),
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
        effect: OngoingEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: Box::new(CardQuery::from_id(site_id)),
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
        effect: OngoingEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: Box::new(CardQuery::from_id(site_id)),
        },
        source: None,
        timestamp: 1,
    });
    state.ongoing_effects.push(TimedOngoingEffect {
        effect: OngoingEffect::RemoveAbilities {
            removal: AbilityRemoval::AllAbilities,
            affected_cards: Box::new(CardQuery::from_id(site_id)),
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

    let target_zone = Location::Square(8, Region::Surface);
    let new_source_location = Location::all_in_region(Region::Surface)
        .into_iter()
        .find(|location| {
            location != &target_zone && !location.get_nearby_sites(&state).contains(&target_zone)
        })
        .expect("a non-nearby source zone should exist");
    Effect::SetCardZone {
        card_id: source_id,
        zone: new_source_location.clone().into(),
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
    state.add_card(Box::new(rubble));

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
    let mut thais = CourtesanThais::new(player_one);
    let thais_id = *thais.get_id();
    thais.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(thais));

    state.queue_one(Effect::TriggerGenesis { card_id: thais_id });
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
