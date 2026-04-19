use crate::{
    card::{
        Ability, ApprenticeWizard, AridDesert, CaptainBaldassare, Card, Enchantress, FootSoldier,
        OgreGoons, Zone, from_name_and_zone,
    },
    deck::Deck,
    effect::{Effect, TokenType},
    networking::message::ServerMessage,
    query::{QueryCache, ZoneQuery},
    state::{Player, PlayerWithDeck, State},
};

/// Creates a test state with proper avatar cards and a live server-message receiver so
/// that `force_sync` calls inside effects do not fail.
fn make_state(zones: Vec<Zone>) -> (State, async_channel::Receiver<ServerMessage>) {
    QueryCache::init();

    let player_one_id = uuid::Uuid::new_v4();
    let player_two_id = uuid::Uuid::new_v4();

    let avatar_one = Enchantress::new(player_one_id);
    let avatar_one_id = *avatar_one.get_id();
    let avatar_two = Enchantress::new(player_two_id);
    let avatar_two_id = *avatar_two.get_id();

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
            "Test".to_string(),
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
            "Test".to_string(),
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

/// Pops and applies all queued effects (mirrors the game loop's `pop_back` order).
async fn drain_effects(state: &mut State) {
    while let Some(effect) = state.effects.pop_back() {
        effect
            .apply(state)
            .await
            .expect("effect should apply without error during drain");
    }
}

#[tokio::test]
async fn test_summon_card_puts_minion_in_target_zone() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.push(Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert_eq!(state.get_card(&id).get_zone(), &Zone::Realm(1));
}

#[tokio::test]
async fn test_summon_card_adds_summoning_sickness_to_minion() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.push(Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        state
            .get_card(&id)
            .has_ability(&state, &Ability::SummoningSickness),
        "minion should have SummoningSickness after SummonCard"
    );
}

#[tokio::test]
async fn test_summon_card_no_summoning_sickness_with_charge() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    let mut minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    minion.add_ability(Ability::Charge);
    state.cards.push(Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        !state
            .get_card(&id)
            .has_ability(&state, &Ability::SummoningSickness),
        "minion with Charge should not receive SummoningSickness"
    );
}

#[tokio::test]
async fn test_summon_card_queues_genesis_effects() {
    // ApprenticeWizard genesis → DrawSpell
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    let wizard = ApprenticeWizard::new(player_id);
    let id = *wizard.get_id();
    state.cards.push(Box::new(wizard));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();

    let has_draw_spell = state
        .effects
        .iter()
        .any(|e| matches!(&**e, Effect::DrawSpell { .. }));
    assert!(
        has_draw_spell,
        "SummonCard should queue genesis effects (DrawSpell for ApprenticeWizard)"
    );
}

#[tokio::test]
async fn test_summon_card_applies_on_summon_effects() {
    // CaptainBaldassare on_summon → AddDeferredEffect
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    let baldassare = CaptainBaldassare::new(player_id);
    let id = *baldassare.get_id();
    state.cards.push(Box::new(baldassare));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert!(
        !state.deferred_effects.is_empty(),
        "on_summon should have registered a deferred effect for CaptainBaldassare"
    );
}

// -------------------------------------------------------------------------
// PlayCard (minion) tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn test_play_card_minion_ends_in_target_zone() {
    // OgreGoons costs 3F; AridDesert in Realm(1) provides fire threshold.
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.compute_world_effects().await.unwrap();

    let mut ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    ogre.set_zone(Zone::Hand);
    state.cards.push(Box::new(ogre));

    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        zone: ZoneQuery::from_zone(Zone::Realm(1)),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&ogre_id).get_zone(),
        &Zone::Realm(1),
        "minion should end in the chosen zone"
    );
}

#[tokio::test]
async fn test_play_card_minion_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.compute_world_effects().await.unwrap();

    let mut ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    ogre.set_zone(Zone::Hand);
    state.cards.push(Box::new(ogre));

    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        zone: ZoneQuery::from_zone(Zone::Realm(1)),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert!(
        state
            .get_card(&ogre_id)
            .has_ability(&state, &Ability::SummoningSickness),
        "minion should have SummoningSickness after being played"
    );
}

#[tokio::test]
async fn test_summon_token_unit_placed_in_target_zone() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldiers: Vec<_> = state
        .cards
        .iter()
        .filter(|c| c.get_name() == FootSoldier::NAME)
        .collect();
    assert_eq!(soldiers.len(), 1, "one FootSoldier token should exist");
    assert_eq!(
        soldiers[0].get_zone(),
        &Zone::Realm(1),
        "FootSoldier should be in the target zone"
    );
}

#[tokio::test]
async fn test_summon_token_unit_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1)]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Realm(1),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldier = state
        .cards
        .iter()
        .find(|c| c.get_name() == FootSoldier::NAME)
        .expect("FootSoldier should exist after SummonToken");
    assert!(
        soldier.has_ability(&state, &Ability::SummoningSickness),
        "summoned unit token should have SummoningSickness"
    );
}
