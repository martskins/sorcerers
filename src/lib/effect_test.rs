use crate::{
    card::{
        Ability, ApprenticeWizard, AridDesert, BottomlessPit, Card, CardStatus, Damage, Drought,
        Enchantress, FootSoldier, OgreGoons, PhaseAssassin, Region, SeaRaider, SpringRiver,
        VaultsOfZul, YourkeCrossbowmen, from_name_and_zone,
    },
    deck::Deck,
    effect::{
        DeferredEffect, DrawKind, Effect, EffectCallback, EffectReplacementCallback,
        TemporaryEffect, TokenType,
    },
    game::Direction,
    networking::message::ServerMessage,
    query::{
        CardQuery, EffectQuery, LocationQuery, QueryCache, ZoneQuery, entered_sites, entered_zones,
    },
    state::{Player, PlayerWithDeck, State},
    zone::{Location, Zone},
};
use std::{collections::HashMap, sync::Arc};

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
async fn test_drawing_from_empty_site_deck_loses_game() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 1,
        kind: DrawKind::Site,
    });
    drain_effects(&mut state).await;

    assert!(
        state.eliminated_players.contains(&player_id),
        "attempting to draw from an empty site deck should lose the game"
    );
}

#[tokio::test]
async fn test_drawing_from_empty_spell_deck_loses_game() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 1,
        kind: DrawKind::Spell,
    });
    drain_effects(&mut state).await;

    assert!(
        state.eliminated_players.contains(&player_id),
        "attempting to draw from an empty spell deck should lose the game"
    );
}

#[tokio::test]
async fn test_plain_strike_does_not_make_target_strike_back() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut striker = OgreGoons::new(player_id);
    let striker_id = *striker.get_id();
    striker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(striker_id, Box::new(striker));

    let mut target = ApprenticeWizard::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(target_id, Box::new(target));

    state.queue_one(Effect::Strike {
        striker_id,
        target_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&striker_id).get_damage_taken().unwrap(),
        0,
        "a plain Strike should not create a defending counterstrike"
    );
    assert_eq!(
        state.get_card(&target_id).get_zone(),
        &Zone::Cemetery,
        "the target should still take strike damage"
    );
}

#[tokio::test]
async fn test_disabled_unit_cannot_strike() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut striker = OgreGoons::new(player_id);
    let striker_id = *striker.get_id();
    striker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    striker.add_status(CardStatus::Disabled);
    state.cards.insert(striker_id, Box::new(striker));

    let mut target = ApprenticeWizard::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(target_id, Box::new(target));

    state.queue_one(Effect::Strike {
        striker_id,
        target_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&target_id).get_damage_taken().unwrap(),
        0,
        "a disabled unit should not strike or deal strike damage"
    );
}

#[tokio::test]
async fn test_ranged_projectile_hits_intervening_unit() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut striker = YourkeCrossbowmen::new(player_id);
    let striker_id = *striker.get_id();
    striker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(striker_id, Box::new(striker));

    let mut blocker = ApprenticeWizard::new(opponent_id);
    let blocker_id = *blocker.get_id();
    blocker.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.cards.insert(blocker_id, Box::new(blocker));

    let mut original_target = ApprenticeWizard::new(opponent_id);
    let original_target_id = *original_target.get_id();
    original_target.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .cards
        .insert(original_target_id, Box::new(original_target));

    state.queue_one(Effect::ShootProjectile {
        id: uuid::Uuid::new_v4(),
        range: Some(2),
        player_id,
        shooter: striker_id,
        from_zone: Zone::Location(Location::Square(1, Region::Surface)),
        direction: Direction::Right,
        damage: 3,
        ranged_strike: true,
        piercing: false,
        splash_damage: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&blocker_id).get_zone(),
        &Zone::Cemetery,
        "the intervening unit should be hit by the ranged projectile"
    );
    assert_eq!(
        state
            .get_card(&original_target_id)
            .get_damage_taken()
            .unwrap(),
        0,
        "the originally targeted unit should not be struck through a blocker"
    );
}

#[tokio::test]
async fn test_ranged_projectile_damage_is_distinct_from_regular_projectile_damage() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut shooter = OgreGoons::new(player_id);
    let shooter_id = *shooter.get_id();
    shooter.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(shooter_id, Box::new(shooter));

    let mut target = YourkeCrossbowmen::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.cards.insert(target_id, Box::new(target));

    state.queue_one(Effect::ShootProjectile {
        id: uuid::Uuid::new_v4(),
        range: Some(1),
        player_id,
        shooter: shooter_id,
        from_zone: Zone::Location(Location::Square(1, Region::Surface)),
        direction: Direction::Right,
        damage: 1,
        ranged_strike: false,
        piercing: false,
        splash_damage: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&target_id).get_damage_taken().unwrap(),
        1,
        "Yourke should still take damage from non-ranged-strike projectiles"
    );

    state.queue_one(Effect::ShootProjectile {
        id: uuid::Uuid::new_v4(),
        range: Some(1),
        player_id,
        shooter: shooter_id,
        from_zone: Zone::Location(Location::Square(1, Region::Surface)),
        direction: Direction::Right,
        damage: 1,
        ranged_strike: true,
        piercing: false,
        splash_damage: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&target_id).get_damage_taken().unwrap(),
        1,
        "Yourke should prevent the ranged-strike damage carried by a projectile"
    );
}

#[tokio::test]
async fn test_effect_description_can_use_removed_card_lookup() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut attacker = ApprenticeWizard::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(attacker_id, Box::new(attacker));

    let mut token = FootSoldier::new(player_id);
    let token_id = *token.get_id();
    token.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.cards.insert(token_id, Box::new(token));

    state.queue_one(Effect::RemoveCardFromGame { card_id: token_id });
    drain_effects(&mut state).await;

    let description = Effect::TakeDamage {
        card_id: token_id,
        from: attacker_id,
        damage: Damage::basic(1),
    }
    .description(&state)
    .await
    .unwrap();

    assert_eq!(
        description,
        Some("Foot Soldier takes 1 damage from Apprentice Wizard".to_string())
    );
}

#[tokio::test]
async fn test_bury_token_removes_token_after_cleanup_effects() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut token = FootSoldier::new(player_id);
    let token_id = *token.get_id();
    token.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(token_id, Box::new(token));

    state.queue_one(Effect::BuryCard { card_id: token_id });
    drain_effects(&mut state).await;

    assert!(
        !state.cards.contains_key(&token_id),
        "token should be removed after its cleanup effects have resolved"
    );
}

#[tokio::test]
async fn test_disabled_unit_does_not_counterstrike_when_attacked() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut attacker = OgreGoons::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(attacker_id, Box::new(attacker));

    let mut defender = ApprenticeWizard::new(opponent_id);
    let defender_id = *defender.get_id();
    defender.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    defender.add_status(CardStatus::Disabled);
    state.cards.insert(defender_id, Box::new(defender));

    state.queue_one(Effect::Attack {
        attacker_id,
        defender_id,
        defending_ids: vec![],
        damage_assignment: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&attacker_id).get_damage_taken().unwrap(),
        0,
        "a disabled defender should not counterstrike"
    );
}

#[tokio::test]
async fn test_multiple_defenders_split_attack_damage() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut attacker = OgreGoons::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(attacker_id, Box::new(attacker));

    let mut defender_one = ApprenticeWizard::new(opponent_id);
    let defender_one_id = *defender_one.get_id();
    defender_one.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.cards.insert(defender_one_id, Box::new(defender_one));

    let mut defender_two = ApprenticeWizard::new(opponent_id);
    let defender_two_id = *defender_two.get_id();
    defender_two.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state.cards.insert(defender_two_id, Box::new(defender_two));

    state.queue_one(Effect::Attack {
        attacker_id,
        defender_id: defender_one_id,
        defending_ids: vec![defender_one_id, defender_two_id],
        damage_assignment: Some(HashMap::from([(defender_one_id, 1), (defender_two_id, 2)])),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&defender_one_id).get_zone(), &Zone::Cemetery);
    assert_eq!(state.get_card(&defender_two_id).get_zone(), &Zone::Cemetery);
    assert_eq!(
        state.get_card(&attacker_id).get_damage_taken().unwrap(),
        2,
        "both surviving-at-resolution defenders should strike back"
    );
}

#[tokio::test]
async fn test_multiple_defender_first_strike_can_stop_split_damage() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut attacker = ApprenticeWizard::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(attacker_id, Box::new(attacker));

    let mut first_striker = ApprenticeWizard::new(opponent_id);
    let first_striker_id = *first_striker.get_id();
    first_striker.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    first_striker
        .get_unit_base_mut()
        .unwrap()
        .abilities
        .push(Ability::FirstStrike);
    state
        .cards
        .insert(first_striker_id, Box::new(first_striker));

    let mut other_defender = ApprenticeWizard::new(opponent_id);
    let other_defender_id = *other_defender.get_id();
    other_defender.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .cards
        .insert(other_defender_id, Box::new(other_defender));

    state.queue_one(Effect::Attack {
        attacker_id,
        defender_id: other_defender_id,
        defending_ids: vec![first_striker_id, other_defender_id],
        damage_assignment: Some(HashMap::from([
            (first_striker_id, 0),
            (other_defender_id, 1),
        ])),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&attacker_id).get_zone(), &Zone::Cemetery);
    assert_eq!(
        state
            .get_card(&other_defender_id)
            .get_damage_taken()
            .unwrap(),
        0,
        "a dead non-first-strike attacker should not deal assigned combat damage"
    );
}

#[test]
fn test_disabled_units_cannot_defend_or_intercept() {
    let (mut state, _rx) = make_state(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
    ]);
    let player_id = state.players[0].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    let mut disabled_defender = FootSoldier::new(player_id);
    let disabled_defender_id = *disabled_defender.get_id();
    disabled_defender.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    disabled_defender.add_status(CardStatus::Disabled);
    state
        .cards
        .insert(disabled_defender_id, Box::new(disabled_defender));

    let mut able_defender = FootSoldier::new(player_id);
    let able_defender_id = *able_defender.get_id();
    able_defender.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state
        .cards
        .insert(able_defender_id, Box::new(able_defender));

    let defenders = state.get_defenders_for_attack(&avatar_id);
    assert!(
        !defenders.contains(&disabled_defender_id),
        "disabled units should not be valid defenders"
    );
    assert!(
        defenders.contains(&able_defender_id),
        "able nearby units should remain valid defenders"
    );

    let opponent_id = state.players[1].id;
    let opponent_avatar_id = state.get_player_avatar_id(&opponent_id).unwrap();
    state
        .get_card_mut(&opponent_avatar_id)
        .set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state
        .get_card_mut(&disabled_defender_id)
        .set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .get_card_mut(&able_defender_id)
        .set_zone(Zone::Location(Location::Square(3, Region::Surface)));

    let interceptors = state.get_interceptors_for_move(
        &[
            Zone::Location(Location::Square(2, Region::Surface)),
            Zone::Location(Location::Square(3, Region::Surface)),
        ],
        &opponent_avatar_id,
        &player_id,
    );
    assert!(
        !interceptors.contains(&disabled_defender_id),
        "disabled units should not be valid interceptors"
    );
    assert!(
        interceptors.contains(&able_defender_id),
        "able units at the final location should remain valid interceptors"
    );
}

#[tokio::test]
async fn test_direct_avatar_damage_after_deaths_door_loses_game() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    let opponent_avatar_id = state.get_player_avatar_id(&opponent_id).unwrap();

    state.queue_one(Effect::TakeDamage {
        card_id: avatar_id,
        from: opponent_avatar_id,
        damage: Damage::basic(20),
    });
    drain_effects(&mut state).await;

    assert!(
        !state.eliminated_players.contains(&player_id),
        "reaching Death's Door should not immediately lose the game"
    );

    state
        .get_card_mut(&avatar_id)
        .get_avatar_base_mut()
        .unwrap()
        .can_die = true;

    state.queue_one(Effect::TakeDamage {
        card_id: avatar_id,
        from: opponent_avatar_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert!(
        state.eliminated_players.contains(&player_id),
        "direct damage to an avatar after Death's Door should be a death blow"
    );
}

#[tokio::test]
async fn test_site_damage_after_deaths_door_is_not_death_blow() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    let opponent_avatar_id = state.get_player_avatar_id(&opponent_id).unwrap();

    let mut site = AridDesert::new(player_id);
    let site_id = *site.get_id();
    site.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(site_id, Box::new(site));

    state.queue_one(Effect::TakeDamage {
        card_id: avatar_id,
        from: opponent_avatar_id,
        damage: Damage::basic(20),
    });
    drain_effects(&mut state).await;

    state
        .get_card_mut(&avatar_id)
        .get_avatar_base_mut()
        .unwrap()
        .can_die = true;

    state.queue_one(Effect::TakeDamage {
        card_id: site_id,
        from: opponent_avatar_id,
        damage: Damage::strike(1, false),
    });
    drain_effects(&mut state).await;

    assert!(
        !state.eliminated_players.contains(&player_id),
        "damage to a site causes avatar life loss, not a death blow"
    );
}

#[tokio::test]
async fn test_vaults_of_zul_triggers_on_stop_not_intermediate_enter() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    let mut vaults = VaultsOfZul::new(player_id);
    let vaults_id = *vaults.get_id();
    vaults.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.cards.insert(vaults_id, Box::new(vaults));

    Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(3, Region::Surface))).with_region(Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Zone::Location(Location::Square(2, Region::Surface)),
            Zone::Location(Location::Square(3, Region::Surface)),
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

    state
        .get_card_mut(&vaults_id)
        .set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(3, Region::Surface))).with_region(Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Zone::Location(Location::Square(2, Region::Surface)),
            Zone::Location(Location::Square(3, Region::Surface)),
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
async fn test_enter_site_triggers_when_card_is_summoned_there() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut pit = BottomlessPit::new(player_id);
    let pit_id = *pit.get_id();
    pit.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(pit_id, Box::new(pit));

    let ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    state.cards.insert(ogre_id, Box::new(ogre));

    state.queue_one(Effect::SummonCards {
        cards: vec![(player_id, ogre_id, Location::Square(1, Region::Surface))],
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&ogre_id).get_zone(),
        &Zone::Cemetery,
        "Bottomless Pit should trigger when a minion is summoned into it"
    );
}

#[tokio::test]
async fn test_phase_assassin_keeps_stealth_after_entering_void() {
    let (mut state, _rx) = make_state(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
    ]);
    let player_id = state.players[0].id;

    let mut assassin = PhaseAssassin::new(player_id);
    let assassin_id = *assassin.get_id();
    assassin.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(assassin_id, Box::new(assassin));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: assassin_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(2, Region::Surface))).with_region(Region::Surface),
        ),
        tap: false,
        through_path: None,
    });
    drain_effects(&mut state).await;

    assert!(
        state
            .get_card(&assassin_id)
            .has_ability(&state, &Ability::Stealth),
        "Phase Assassin should gain Stealth when entering the void"
    );

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: assassin_id,
        from: Location::Square(2, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(3, Region::Surface))).with_region(Region::Surface),
        ),
        tap: false,
        through_path: None,
    });
    drain_effects(&mut state).await;

    assert!(
        state
            .get_card(&assassin_id)
            .has_ability(&state, &Ability::Stealth),
        "Phase Assassin's gained Stealth should not disappear just because it leaves the void"
    );
}

#[tokio::test]
async fn test_teleport_triggers_visit_zone_once() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut assassin = PhaseAssassin::new(player_id);
    let assassin_id = *assassin.get_id();
    assassin.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(assassin_id, Box::new(assassin));

    state.queue_one(Effect::TeleportCard {
        player_id,
        card_id: assassin_id,
        to_location: Location::Square(2, Region::Surface),
    });
    drain_effects(&mut state).await;

    let stealth_counters = state
        .get_card(&assassin_id)
        .get_unit_base()
        .expect("Phase Assassin should have unit base")
        .ability_counters
        .iter()
        .filter(|counter| counter.ability == Ability::Stealth)
        .count();

    assert_eq!(
        stealth_counters, 1,
        "teleport should let MoveCard trigger on_visit_zone exactly once"
    );
}

#[tokio::test]
async fn test_region_changes_enter_location_but_not_site() {
    let (state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let card_id = state.get_player_avatar_id(&player_id).unwrap();

    let effect = Effect::MoveCard {
        player_id,
        card_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(1, Region::Underground)))
                .with_region(Region::Underground),
        ),
        tap: false,
        through_path: None,
    };

    assert_eq!(
        entered_zones(&effect, &state).await.unwrap(),
        vec![(
            card_id,
            Zone::Location(Location::Square(1, Region::Underground))
        )],
        "changing regions on the same realm square should count as entering a new location"
    );
    assert!(
        entered_sites(&effect, &state).await.unwrap().is_empty(),
        "changing regions on the same realm square should not count as entering a new site"
    );
}

#[tokio::test]
async fn test_minion_without_burrowing_dies_underground() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    apprentice_wizard.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state
        .cards
        .insert(apprentice_wizard_id, Box::new(apprentice_wizard));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: apprentice_wizard_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_zone(
            (Zone::Location(Location::Square(1, Region::Surface))).with_region(Region::Underground),
        ),
        tap: false,
        through_path: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&apprentice_wizard_id).get_zone(),
        &Zone::Cemetery
    );
}

#[tokio::test]
async fn test_minion_without_voidwalk_is_banished_in_void() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    state
        .cards
        .insert(apprentice_wizard_id, Box::new(apprentice_wizard));

    state.queue_one(Effect::SummonCards {
        cards: vec![(
            player_id,
            apprentice_wizard_id,
            Location::Square(1, Region::Void),
        )],
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&apprentice_wizard_id).get_zone(),
        &Zone::Banish
    );
}

#[tokio::test]
async fn test_location_survival_is_checked_when_site_type_changes() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    apprentice_wizard.add_ability(Ability::Burrowing);
    apprentice_wizard.set_zone(Zone::Location(Location::Square(1, Region::Underground)));
    state
        .cards
        .insert(apprentice_wizard_id, Box::new(apprentice_wizard));

    state.queue_one(Effect::AddTemporaryEffect {
        effect: TemporaryEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: CardQuery::new().sites(),
            expires_on_effect: EffectQuery::TurnEnd { player_id: None },
        },
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&apprentice_wizard_id).get_zone(),
        &Zone::Cemetery
    );
}

#[tokio::test]
async fn test_submerged_minion_dies_when_water_site_becomes_land() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut spring_river = SpringRiver::new(player_id);
    let spring_river_id = *spring_river.get_id();
    spring_river.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.cards.insert(spring_river_id, Box::new(spring_river));

    let mut apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    apprentice_wizard.add_ability(Ability::Submerge);
    apprentice_wizard.set_zone(Zone::Location(Location::Square(1, Region::Underwater)));
    state
        .cards
        .insert(apprentice_wizard_id, Box::new(apprentice_wizard));

    let mut drought = Drought::new(player_id);
    let drought_id = *drought.get_id();
    drought.set_zone(Zone::Hand);
    state.cards.insert(drought_id, Box::new(drought));

    state.queue_one(Effect::SetCardZone {
        card_id: drought_id,
        zone: Zone::Location(Location::Square(1, Region::Surface)),
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&apprentice_wizard_id).get_zone(),
        &Zone::Cemetery
    );
}

#[tokio::test]
async fn test_site_generates_mana_when_set_card_zone_enters_realm() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let spring_river = SpringRiver::new(player_id);
    let spring_river_id = *spring_river.get_id();
    state.cards.insert(spring_river_id, Box::new(spring_river));

    state.queue_one(Effect::SetCardZone {
        card_id: spring_river_id,
        zone: Zone::Location(Location::Square(1, Region::Surface)),
    });
    drain_effects(&mut state).await;

    assert_eq!(
        *state.get_player_mana_mut(&player_id),
        1,
        "a site entering the realm on its controller's turn should provide mana"
    );
}

#[tokio::test]
async fn test_site_entering_realm_outside_controller_turn_does_not_generate_mana() {
    let (mut state, _rx) = make_state(vec![]);
    let opponent_id = state.players[1].id;

    let spring_river = SpringRiver::new(opponent_id);
    let spring_river_id = *spring_river.get_id();
    state.cards.insert(spring_river_id, Box::new(spring_river));

    state.queue_one(Effect::SetCardZone {
        card_id: spring_river_id,
        zone: Zone::Location(Location::Square(1, Region::Surface)),
    });
    drain_effects(&mut state).await;

    assert_eq!(
        *state.get_player_mana_mut(&opponent_id),
        0,
        "a site should only provide mana when it enters during its controller's turn"
    );
}

#[tokio::test]
async fn test_played_site_generates_mana_once() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let spring_river = SpringRiver::new(player_id);
    let spring_river_id = *spring_river.get_id();
    state.cards.insert(spring_river_id, Box::new(spring_river));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: spring_river_id,
        zone: ZoneQuery::from_zone(Zone::Location(Location::Square(1, Region::Surface))),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(
        *state.get_player_mana_mut(&player_id),
        1,
        "playing a site should use the generic realm-entry mana path exactly once"
    );
}

#[tokio::test]
async fn test_temporary_modify_effect_runs_before_handler_and_expires() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let draw_query = EffectQuery::DrawCard { player_id: None };
    let convert_draw_to_mana: EffectReplacementCallback = Arc::new(|_state, effect| {
        Box::pin(async move {
            if let Effect::DrawCard { player_id, .. } = effect {
                *effect = Effect::AdjustMana {
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
        kind: DrawKind::Choice,
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
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AdjustMana {
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
        kind: DrawKind::Choice,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 1);
    assert!(state.deferred_effects().is_empty());
}

#[tokio::test]
async fn test_deferred_multitrigger_remains_after_trigger() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AdjustMana {
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
        kind: DrawKind::Choice,
    });
    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
        kind: DrawKind::Choice,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 2);
    assert_eq!(state.deferred_effects().len(), 1);
}

#[tokio::test]
async fn test_deferred_expiry_removes_without_triggering() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let grant_mana: EffectCallback = Arc::new(|_state, source_id, _effect| {
        Box::pin(async move {
            Ok(vec![Effect::AdjustMana {
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
        kind: DrawKind::Choice,
    });
    drain_effects(&mut state).await;

    assert_eq!(*state.get_player_mana_mut(&player_id), 0);
    assert!(state.deferred_effects().is_empty());
}

#[tokio::test]
async fn test_temporary_expiry_removes_after_matching_resolved_effect() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let site_id = *state
        .cards
        .values()
        .find(|card| card.is_site())
        .expect("test state should have a site")
        .get_id();

    state
        .temporary_effects_mut()
        .push(TemporaryEffect::GrantAbility {
            ability: Ability::Flooded,
            affected_cards: CardQuery::from_id(site_id),
            expires_on_effect: EffectQuery::DrawCard { player_id: None },
        });

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
        kind: DrawKind::Choice,
    });
    drain_effects(&mut state).await;

    assert!(state.temporary_effects().is_empty());
}

#[tokio::test]
async fn test_summon_card_puts_minion_in_target_zone() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCards {
        cards: vec![(player_id, id, Location::Square(1, Region::Surface))],
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert_eq!(
        state.get_card(&id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface))
    );
}

#[tokio::test]
async fn test_summon_card_adds_summoning_sickness_to_minion() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCards {
        cards: vec![(player_id, id, Location::Square(1, Region::Surface))],
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        state
            .get_card(&id)
            .has_status(&state, &CardStatus::SummoningSickness),
        "minion should have SummoningSickness after SummonCard"
    );
}

#[tokio::test]
async fn test_summon_card_no_summoning_sickness_with_charge() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut minion = OgreGoons::new(player_id);
    let id = *minion.get_id();
    minion.add_ability(Ability::Charge);
    state.cards.insert(id, Box::new(minion));

    Effect::SummonCards {
        cards: vec![(player_id, id, Location::Square(1, Region::Surface))],
    }
    .apply(&mut state)
    .await
    .unwrap();

    assert!(
        !state
            .get_card(&id)
            .has_status(&state, &CardStatus::SummoningSickness),
        "minion with Charge should not receive SummoningSickness"
    );
}

#[tokio::test]
async fn test_summon_card_queues_genesis_effects() {
    // ApprenticeWizard genesis -> draw spell
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let wizard = ApprenticeWizard::new(player_id);
    let id = *wizard.get_id();
    state.cards.insert(id, Box::new(wizard));

    Effect::SummonCards {
        cards: vec![(player_id, id, Location::Square(1, Region::Surface))],
    }
    .apply(&mut state)
    .await
    .unwrap();

    let has_draw_spell = state.effects.iter().any(|e| {
        matches!(
            *e,
            Effect::DrawCard {
                kind: DrawKind::Spell,
                ..
            }
        )
    });
    assert!(
        has_draw_spell,
        "SummonCards should queue genesis effects (draw spell for ApprenticeWizard)"
    );
}

#[tokio::test]
async fn test_played_site_genesis_can_target_itself() {
    QueryCache::init();

    let game_id = uuid::Uuid::new_v4();
    let player_one_id = uuid::Uuid::new_v4();
    let player_two_id = uuid::Uuid::new_v4();

    let mut desert = AridDesert::new(player_one_id);
    let desert_id = *desert.get_id();
    desert.set_zone(Zone::Hand);

    let avatar_one = Enchantress::new(player_one_id);
    let avatar_one_id = *avatar_one.get_id();
    let avatar_two = Enchantress::new(player_two_id);
    let avatar_two_id = *avatar_two.get_id();

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
        cards: vec![Box::new(desert), Box::new(avatar_one)],
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
    let (client_tx, client_rx) = async_channel::unbounded();
    let mut state = State::new(game_id, vec![player1, player2], server_tx, client_rx);

    tokio::spawn(async move {
        let mut answered_pick = false;
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickCard {
                    player_id, cards, ..
                } => {
                    assert!(
                        cards.contains(&desert_id),
                        "Genesis target choices should include the site that just entered"
                    );
                    client_tx
                        .send(crate::networking::message::ClientMessage::PickCard {
                            game_id,
                            player_id,
                            card_id: desert_id,
                        })
                        .await
                        .unwrap();
                    answered_pick = true;
                }
                ServerMessage::Resume { .. } if answered_pick => break,
                _ => {}
            }
        }
    });

    Effect::PlayCard {
        player_id: player_one_id,
        card_id: desert_id,
        zone: ZoneQuery::from_zone(Zone::Location(Location::Square(1, Region::Surface))),
        spellcaster: avatar_one_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_summon_card_applies_on_summon_effects() {
    // Sea Raider on_summon → AddDeferredEffect
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let sea_raider = SeaRaider::new(player_id);
    let id = *sea_raider.get_id();
    state.cards.insert(id, Box::new(sea_raider));

    Effect::SummonCards {
        cards: vec![(player_id, id, Location::Square(1, Region::Surface))],
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
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.reconcile_ongoing_effects_for_test().await.unwrap();

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
        zone: ZoneQuery::from_zone(Zone::Location(Location::Square(1, Region::Surface))),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&ogre_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface)),
        "minion should end in the chosen zone"
    );
}

#[tokio::test]
async fn test_play_card_minion_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;
    state.reconcile_ongoing_effects_for_test().await.unwrap();

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
        zone: ZoneQuery::from_zone(Zone::Location(Location::Square(1, Region::Surface))),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert!(
        state
            .get_card(&ogre_id)
            .has_status(&state, &CardStatus::SummoningSickness),
        "minion should have SummoningSickness after being played"
    );
}

#[tokio::test]
async fn test_summon_token_unit_placed_in_target_zone() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Location(Location::Square(1, Region::Surface)),
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
        &Zone::Location(Location::Square(1, Region::Surface)),
        "FootSoldier should be in the target zone"
    );
}

#[tokio::test]
async fn test_summon_token_unit_has_summoning_sickness() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    Effect::SummonToken {
        player_id,
        token_type: TokenType::FootSoldier,
        zone: Zone::Location(Location::Square(1, Region::Surface)),
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
        soldier.has_status(&state, &CardStatus::SummoningSickness),
        "summoned unit token should have SummoningSickness"
    );
}
