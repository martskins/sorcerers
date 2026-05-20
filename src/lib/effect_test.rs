use crate::{
    card::{
        Ability, ApprenticeWizard, AridDesert, Card, Enchantress, FootSoldier, OgreGoons, Region,
        SeaRaider, VaultsOfZul, from_name_and_zone,
    },
    deck::Deck,
    effect::{
        DeferredEffect, Effect, EffectCallback, EffectReplacementCallback, TemporaryEffect,
        TokenType,
    },
    networking::message::ServerMessage,
    query::{CardQuery, EffectQuery, QueryCache, ZoneQuery},
    state::{Player, PlayerWithDeck, State},
    zone::Zone,
};
use std::sync::Arc;

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
    state
        .apply_effects_without_log()
        .await
        .expect("effect queue should drain without error");
}

#[tokio::test]
async fn test_vaults_of_zul_triggers_on_stop_not_intermediate_enter() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Realm(1, Region::Surface));

    let mut vaults = VaultsOfZul::new(player_id);
    let vaults_id = *vaults.get_id();
    vaults.set_zone(Zone::Realm(2, Region::Surface));
    state.cards.insert(vaults_id, Box::new(vaults));

    Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Zone::Realm(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Realm(3, Region::Surface)),
        tap: false,
        region: Region::Surface,
        through_path: Some(vec![
            Zone::Realm(2, Region::Surface),
            Zone::Realm(3, Region::Surface),
        ]),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        !state
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::SkipNextTurn { player_id: skipped } if skipped == &player_id)),
        "Vaults of Zul should not trigger when an Avatar only enters it mid-movement"
    );

    state.get_card_mut(&vaults_id).set_zone(Zone::Realm(3, Region::Surface));
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Realm(1, Region::Surface));
    Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Zone::Realm(1, Region::Surface),
        to: ZoneQuery::from_zone(Zone::Realm(3, Region::Surface)),
        tap: false,
        region: Region::Surface,
        through_path: Some(vec![
            Zone::Realm(2, Region::Surface),
            Zone::Realm(3, Region::Surface),
        ]),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        state
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::SkipNextTurn { player_id: skipped } if skipped == &player_id)),
        "Vaults of Zul should trigger when an Avatar stops there"
    );
}

#[tokio::test]
async fn test_temporary_modify_effect_runs_before_handler_and_expires() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let draw_query = EffectQuery::DrawCard { player_id: None };
    let convert_draw_to_mana: EffectReplacementCallback = Arc::new(|_state, effect| {
        Box::pin(async move {
            if let Effect::DrawCard { player_id, .. } = effect {
                *effect = Effect::AddMana {
                    player_id: *player_id,
                    mana: 3,
                };
            }
            Ok(())
        })
    });
    state
        .temporary_effects_mut()
        .push(TemporaryEffect::ModifyEffect {
            trigger_on_effect: draw_query.clone(),
            expires_on_effect: draw_query,
            on_effect: convert_draw_to_mana,
        });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 1,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 3);
    assert!(
        state.temporary_effects().is_empty(),
        "temporary modifier should expire after the matching resolved effect"
    );
}

#[tokio::test]
async fn test_deferred_one_shot_removes_itself_after_trigger() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AddMana {
                player_id: *source_id,
                mana: 1,
            }])
        })
    });

    state.deferred_effects_mut().push(DeferredEffect {
        trigger_on_effect: EffectQuery::DrawCard { player_id: None },
        expires_on_effect: None,
        on_effect: grant_mana,
        multitrigger: false,
    });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 1);
    assert!(state.deferred_effects().is_empty());
}

#[tokio::test]
async fn test_deferred_multitrigger_remains_after_trigger() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AddMana {
                player_id: *source_id,
                mana: 1,
            }])
        })
    });

    state.deferred_effects_mut().push(DeferredEffect {
        trigger_on_effect: EffectQuery::DrawCard { player_id: None },
        expires_on_effect: None,
        on_effect: grant_mana,
        multitrigger: true,
    });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
    });
    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 2);
    assert_eq!(state.deferred_effects().len(), 1);
}

#[tokio::test]
async fn test_deferred_expiry_removes_without_triggering() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AddMana {
                player_id: *source_id,
                mana: 1,
            }])
        })
    });

    state.deferred_effects_mut().push(DeferredEffect {
        trigger_on_effect: EffectQuery::TurnStart { player_id: None },
        expires_on_effect: Some(EffectQuery::DrawCard { player_id: None }),
        on_effect: grant_mana,
        multitrigger: false,
    });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 0);
    assert!(state.deferred_effects().is_empty());
}

#[tokio::test]
async fn test_temporary_expiry_removes_after_matching_resolved_effect() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    let site_id = *state
        .cards
        .values()
        .find(|card| card.is_site())
        .expect("test state should have a site")
        .get_id();

    state
        .temporary_effects_mut()
        .push(TemporaryEffect::FloodSites {
            affected_sites: CardQuery::from_id(site_id),
            expires_on_effect: EffectQuery::DrawCard { player_id: None },
        });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
    });
    drain_effects(&mut state).await;

    assert!(state.temporary_effects().is_empty());
}

#[tokio::test]
async fn test_summon_card_puts_minion_in_target_zone() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert_eq!(
        state.get_card(&id).get_zone(),
        &Zone::Realm(1, Region::Surface)
    );
}

#[tokio::test]
async fn test_summon_card_adds_summoning_sickness_to_minion() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1, Region::Surface),
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
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let mut minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    minion.add_ability(Ability::Charge);
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1, Region::Surface),
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
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let wizard = ApprenticeWizard::new(player_id);
    let id = *wizard.get_id();
    state.cards.insert(id, Box::new(wizard));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();

    let has_draw_spell = state
        .effects
        .iter()
        .any(|e| matches!(*e, Effect::DrawSpell { .. }));
    assert!(
        has_draw_spell,
        "SummonCard should queue genesis effects (DrawSpell for ApprenticeWizard)"
    );
}

#[tokio::test]
async fn test_summon_card_applies_on_summon_effects() {
    // Sea Raider on_summon → AddDeferredEffect
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    let sea_raider = SeaRaider::new(player_id);
    let id = *sea_raider.get_id();
    state.cards.insert(id, Box::new(sea_raider));

    Effect::SummonCard {
        player_id,
        card_id: id,
        zone: Zone::Realm(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert!(
        !state.deferred_effects().is_empty(),
        "on_summon should have registered a deferred effect for Sea Raider"
    );
}

// -------------------------------------------------------------------------
// PlayCard (minion) tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn test_play_card_minion_ends_in_target_zone() {
    // OgreGoons costs 3F; AridDesert in Realm(1) provides fire threshold.
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.compute_world_effects().await.unwrap();

    let mut ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    ogre.set_zone(Zone::Hand);
    state.cards.insert(ogre_id, Box::new(ogre));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        zone: ZoneQuery::from_zone(Zone::Realm(1, Region::Surface)),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&ogre_id).get_zone(),
        &Zone::Realm(1, Region::Surface),
        "minion should end in the chosen zone"
    );
}

#[tokio::test]
async fn test_play_card_minion_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.compute_world_effects().await.unwrap();

    let mut ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    ogre.set_zone(Zone::Hand);
    state.cards.insert(ogre_id, Box::new(ogre));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        zone: ZoneQuery::from_zone(Zone::Realm(1, Region::Surface)),
        spellcaster: avatar_id,
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
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Realm(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldiers: Vec<_> = state
        .cards
        .values()
        .filter(|c| c.get_name() == FootSoldier::NAME)
        .collect();
    assert_eq!(soldiers.len(), 1, "one FootSoldier token should exist");
    assert_eq!(
        soldiers[0].get_zone(),
        &Zone::Realm(1, Region::Surface),
        "FootSoldier should be in the target zone"
    );
}

#[tokio::test]
async fn test_summon_token_unit_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Realm(1, Region::Surface)]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Realm(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldier = state
        .cards
        .values()
        .find(|c| c.get_name() == FootSoldier::NAME)
        .expect("FootSoldier should exist after SummonToken");
    assert!(
        soldier.has_ability(&state, &Ability::SummoningSickness),
        "summoned unit token should have SummoningSickness"
    );
}
