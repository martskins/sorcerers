use crate::{
    card::*,
    deck::Deck,
    effect::{
        DeferredEffect, DrawKind, Effect, EffectReplacementCallback, FightContext, SummonCard,
        TemporaryEffect, TokenType,
    },
    game::{ActivatedAbility, Direction, UnitAction},
    networking::message::{ClientMessage, ServerMessage},
    query::{CardQuery, EffectQuery, LocationQuery, QueryCache},
    state::{Player, PlayerWithDeck, State},
    zone::{Location, Zone},
};
use std::{collections::HashMap, sync::Arc};

const TEST_HOOK_SOURCE_ID: HookId = 1000;

#[derive(Debug, Clone)]
struct TestHookSource {
    card_base: CardBase,
    source_zones: HookSourceZones,
}

impl TestHookSource {
    fn new(owner_id: uuid::Uuid, source_zones: HookSourceZones) -> Self {
        Self {
            card_base: CardBase {
                id: uuid::Uuid::new_v4(),
                owner_id,
                controller_id: owner_id,
                zone: Zone::Hand,
                ..Default::default()
            },
            source_zones,
        }
    }
}

#[async_trait::async_trait]
impl Card for TestHookSource {
    fn get_name(&self) -> &str {
        "Test Hook Source"
    }

    fn get_description(&self) -> &str {
        ""
    }

    fn get_base_mut(&mut self) -> &mut CardBase {
        &mut self.card_base
    }

    fn get_base(&self) -> &CardBase {
        &self.card_base
    }

    fn hooks(&self, _state: &State) -> anyhow::Result<Vec<Hook>> {
        Ok(vec![Hook {
            id: TEST_HOOK_SOURCE_ID,
            trigger: EffectQuery::DrawCard { player_id: None },
            timing: HookTiming::After,
            source_zones: self.source_zones.clone(),
        }])
    }

    async fn resolve_hook(
        &self,
        hook_id: HookId,
        state: &State,
        _effect: &Effect,
    ) -> anyhow::Result<Vec<Effect>> {
        match hook_id {
            TEST_HOOK_SOURCE_ID => Ok(vec![Effect::AdjustMana {
                player_id: self.get_controller_id(state),
                mana: 1,
            }]),
            _ => Ok(vec![]),
        }
    }
}

/// Creates a test state with proper avatar cards and a live server-message receiver so
/// that `force_sync` calls inside effects do not fail.
fn make_state(zones: Vec<Zone>) -> (State, async_channel::Receiver<ServerMessage>) {
    let (state, server_rx, _client_tx) = make_state_with_client(zones);
    (state, server_rx)
}

fn make_state_with_client(
    zones: Vec<Zone>,
) -> (
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
    let (client_tx, client_rx) = async_channel::unbounded();
    let state = State::new(
        uuid::Uuid::new_v4(),
        vec![player1, player2],
        server_tx,
        client_rx,
    );
    (state, server_rx, client_tx)
}

/// Pops and applies all queued effects (mirrors the game loop's `pop_back` order).
async fn drain_effects(state: &mut State) {
    state
        .apply_effects_without_log()
        .await
        .expect("effect queue should drain without error");
}

// TODO: Fix this test
// #[tokio::test]
// async fn test_enter_zone_matches_played_aura() {
//     let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(8, Region::Surface))]);
//     let player_id = state.players[0].id;
//     let wildfire = Wildfire::new(player_id);
//     let wildfire_id = *wildfire.get_id();
//     let zone = Zone::Location(Location::Square(8, Region::Surface));
//
//     state.add_card(Box::new(wildfire));
//     state.get_card_mut(&wildfire_id).set_zone(zone.clone());
//
//     let effect = Effect::PlayCard {
//         player_id,
//         card_id: wildfire_id,
//         location: LocationQuery::from_zone(zone.clone()),
//         spellcaster: state.get_player_avatar_id(&player_id).unwrap(),
//     };
//     let query = EffectQuery::EnterLocation {
//         card: CardQuery::from_id(wildfire_id),
//         location: LocationQuery::from_zone(zone),
//         from: None,
//     };
//
//     assert!(query.matches(&effect, &state).await.unwrap());
//     assert_eq!(
//         query.source_ids(&effect, &state).await.unwrap(),
//         vec![wildfire_id]
//     );
// }

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
    state.add_card(Box::new(striker));

    let mut target = ApprenticeWizard::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(target));

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
    state.add_card(Box::new(striker));

    let mut target = ApprenticeWizard::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(target));

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
    state.add_card(Box::new(striker));

    let mut blocker = ApprenticeWizard::new(opponent_id);
    let blocker_id = *blocker.get_id();
    blocker.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(blocker));

    let mut original_target = ApprenticeWizard::new(opponent_id);
    let original_target_id = *original_target.get_id();
    original_target.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state.add_card(Box::new(original_target));

    state.queue_one(Effect::ShootProjectile {
        id: uuid::Uuid::new_v4(),
        range: Some(2),
        player_id,
        shooter: striker_id,
        origin: Location::Square(1, Region::Surface),
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
    state.add_card(Box::new(shooter));

    let mut target = YourkeCrossbowmen::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(target));

    state.queue_one(Effect::ShootProjectile {
        id: uuid::Uuid::new_v4(),
        range: Some(1),
        player_id,
        shooter: shooter_id,
        origin: Location::Square(1, Region::Surface),
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
        origin: Location::Square(1, Region::Surface),
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
async fn test_replace_hook_replaces_original_effect() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut demon = DeadOfNightDemon::new(opponent_id);
    let demon_id = *demon.get_id();
    demon.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(demon));

    let mut templar = SirianTemplar::new(player_id);
    let templar_id = *templar.get_id();
    templar.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(templar));

    state.queue_one(Effect::TakeDamage {
        card_id: templar_id,
        from: demon_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&templar_id).get_damage_taken().unwrap(),
        0,
        "Sirian Templar should replace damage from Demon minions"
    );
}

#[tokio::test]
async fn test_turn_end_projectile_damage_is_cleared_before_next_turn() {
    let (mut state, server_rx, client_tx) = make_state_with_client(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
    ]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut dragonettes = ColickyDragonettes::new(player_id);
    let dragonettes_id = *dragonettes.get_id();
    dragonettes.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(dragonettes));

    let mut target = OgreGoons::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(target));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickDirection { player_id, .. } => {
                    client_tx
                        .send(ClientMessage::PickDirection {
                            game_id,
                            player_id,
                            direction: Direction::Right,
                        })
                        .await
                        .unwrap();
                }
                ServerMessage::PickAction { player_id, .. } => {
                    client_tx
                        .send(ClientMessage::PickAction {
                            game_id,
                            player_id,
                            action_idx: 0,
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    state.queue_one(Effect::EndTurn { player_id });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&target_id).get_damage_taken().unwrap(),
        0,
        "damage from end-turn projectiles should not persist into the next turn"
    );
}

#[tokio::test]
async fn test_hook_source_zones_control_out_of_play_triggers() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let in_play_only = TestHookSource::new(player_id, HookSourceZones::InPlay);
    let in_play_only_id = *in_play_only.get_id();
    state.add_card(Box::new(in_play_only));

    let any_zone = TestHookSource::new(player_id, HookSourceZones::Any);
    let any_zone_id = *any_zone.get_id();
    state.add_card(Box::new(any_zone));

    let hand_zone = TestHookSource::new(player_id, HookSourceZones::Zones(vec![Zone::Hand]));
    let hand_zone_id = *hand_zone.get_id();
    state.add_card(Box::new(hand_zone));

    state.queue_one(Effect::DrawCard {
        player_id,
        count: 0,
        kind: DrawKind::Spell,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        *state.get_player_mana_mut(&player_id),
        2,
        "only Any and explicit Hand hook sources should trigger from hand"
    );
}

#[tokio::test]
async fn test_scourge_zombies_triggers_from_cemetery() {
    let (mut state, server_rx, client_tx) =
        make_state_with_client(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut source = FootSoldier::new(opponent_id);
    let source_id = *source.get_id();
    source.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(source));

    let mut mortal = FootSoldier::new(player_id);
    let mortal_id = *mortal.get_id();
    mortal.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(mortal));

    let mut scourge = ScourgeZombies::new(player_id);
    let scourge_id = *scourge.get_id();
    scourge.set_zone(Zone::Cemetery);
    state.add_card(Box::new(scourge));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            if let ServerMessage::PickAction {
                player_id, actions, ..
            } = message
            {
                assert_eq!(actions[0], "Yes");
                client_tx
                    .send(ClientMessage::PickAction {
                        game_id,
                        player_id,
                        action_idx: 0,
                    })
                    .await
                    .unwrap();
            }
        }
    });

    state.queue_one(Effect::KillMinion {
        card_id: mortal_id,
        killer_id: source_id,
        from_attack: false,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&scourge_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface))
    );
    assert!(state.get_card(&scourge_id).is_tapped());
}

#[tokio::test]
async fn test_rimland_nomads_replace_desert_damage() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut desert = AridDesert::new(player_id);
    let desert_id = *desert.get_id();
    desert.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(desert));

    let mut nomads = RimlandNomads::new(player_id);
    let nomads_id = *nomads.get_id();
    nomads.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(nomads));

    state.queue_one(Effect::TakeDamage {
        card_id: nomads_id,
        from: desert_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&nomads_id).get_damage_taken().unwrap(), 0);
}

#[tokio::test]
async fn test_askelon_phoenix_replaces_fire_damage_with_power_counter() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut desert = AridDesert::new(player_id);
    let desert_id = *desert.get_id();
    desert.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(desert));

    let mut phoenix = AskelonPhoenix::new(player_id);
    let phoenix_id = *phoenix.get_id();
    phoenix.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(phoenix));

    state.queue_one(Effect::TakeDamage {
        card_id: phoenix_id,
        from: desert_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    let phoenix = state.get_card(&phoenix_id);
    assert_eq!(phoenix.get_damage_taken().unwrap(), 0);
    assert_eq!(phoenix.get_power(&state).unwrap(), Some(5));
}

#[tokio::test]
async fn test_sling_pixies_replace_damage_from_units_with_four_or_more_power() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut phoenix = AskelonPhoenix::new(opponent_id);
    let phoenix_id = *phoenix.get_id();
    phoenix.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(phoenix));

    let mut pixies = SlingPixies::new(player_id);
    let pixies_id = *pixies.get_id();
    pixies.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(pixies));

    state.queue_one(Effect::TakeDamage {
        card_id: pixies_id,
        from: phoenix_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&pixies_id).get_damage_taken().unwrap(), 0);
}

#[tokio::test]
async fn test_tufted_turtles_replace_first_damage_each_turn() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut source = FootSoldier::new(opponent_id);
    let source_id = *source.get_id();
    source.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(source));

    let mut turtles = TuftedTurtles::new(player_id);
    let turtles_id = *turtles.get_id();
    turtles.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(turtles));

    state.queue_one(Effect::TakeDamage {
        card_id: turtles_id,
        from: source_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&turtles_id).get_damage_taken().unwrap(), 0);

    state.queue_one(Effect::TakeDamage {
        card_id: turtles_id,
        from: source_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&turtles_id).get_damage_taken().unwrap(), 1);
}

#[tokio::test]
async fn test_gilded_aegis_replace_bearer_death() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut source = FootSoldier::new(opponent_id);
    let source_id = *source.get_id();
    source.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(source));

    let mut bearer = FootSoldier::new(player_id);
    let bearer_id = *bearer.get_id();
    bearer.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(bearer));

    let mut aegis = GildedAegis::new(player_id);
    let aegis_id = *aegis.get_id();
    aegis.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    aegis.set_bearer_id(Some(bearer_id));
    state.add_card(Box::new(aegis));

    state.queue_one(Effect::KillMinion {
        card_id: bearer_id,
        killer_id: source_id,
        from_attack: false,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&bearer_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Surface))
    );
    assert_eq!(state.get_card(&aegis_id).get_zone(), &Zone::Banish);
}

#[tokio::test]
async fn test_tvinnax_berserker_untaps_after_attack_kill_without_replacing_kill() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut tvinnax = TvinnaxBerserker::new(player_id);
    let tvinnax_id = *tvinnax.get_id();
    tvinnax.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    tvinnax.set_tapped(true);
    state.add_card(Box::new(tvinnax));

    let mut target = FootSoldier::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(target));

    state.queue_one(Effect::KillMinion {
        card_id: target_id,
        killer_id: tvinnax_id,
        from_attack: true,
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&target_id).get_zone(), &Zone::Cemetery);
    assert!(!state.get_card(&tvinnax_id).is_tapped());
}

#[tokio::test]
async fn test_dodge_roll_replacement_triggers_once_for_multiple_copies() {
    let (mut state, server_rx, client_tx) = make_state_with_client(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
    ]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let origin = Location::Square(1, Region::Surface);
    let destination = Location::Square(2, Region::Surface);

    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(origin.clone().into());

    let mut attacker = FootSoldier::new(opponent_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(origin.clone().into());
    state.add_card(Box::new(attacker));

    let mut defender = FootSoldier::new(player_id);
    let defender_id = *defender.get_id();
    defender.set_zone(origin.clone().into());
    state.add_card(Box::new(defender));

    let mut first_dodge_roll = DodgeRoll::new(player_id);
    let first_dodge_roll_id = *first_dodge_roll.get_id();
    first_dodge_roll.set_zone(Zone::Hand);
    state.add_card(Box::new(first_dodge_roll));

    let mut second_dodge_roll = DodgeRoll::new(player_id);
    let second_dodge_roll_id = *second_dodge_roll.get_id();
    second_dodge_roll.set_zone(Zone::Hand);
    state.add_card(Box::new(second_dodge_roll));

    let picked_destination = destination.clone();
    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickAction {
                    player_id, actions, ..
                } => {
                    assert_eq!(actions[0], "Yes");
                    client_tx
                        .send(ClientMessage::PickAction {
                            game_id,
                            player_id,
                            action_idx: 0,
                        })
                        .await
                        .unwrap();
                }
                ServerMessage::PickLocation {
                    player_id,
                    locations,
                    ..
                } => {
                    assert!(locations.contains(&picked_destination));
                    client_tx
                        .send(ClientMessage::PickLocation {
                            game_id,
                            player_id,
                            location: picked_destination.clone(),
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    state.queue_one(Effect::DeclareAttack {
        attacker_id,
        target_id: defender_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&defender_id).get_location(), &destination);
    assert_eq!(state.get_card(&attacker_id).get_location(), &origin);

    let dodge_roll_zones = [
        state.get_card(&first_dodge_roll_id).get_zone().clone(),
        state.get_card(&second_dodge_roll_id).get_zone().clone(),
    ];
    let cemetery_count = dodge_roll_zones
        .iter()
        .filter(|zone| **zone == Zone::Cemetery)
        .count();
    let hand_count = dodge_roll_zones
        .iter()
        .filter(|zone| **zone == Zone::Hand)
        .count();
    assert_eq!(
        cemetery_count, 1,
        "only one Dodge Roll should be cast for a single attack"
    );
    assert_eq!(
        hand_count, 1,
        "extra Dodge Roll copies should not also resolve the same attack"
    );
}

#[tokio::test]
async fn test_royal_bodyguard_replace_nearby_avatar_damage() {
    let (mut state, server_rx, client_tx) =
        make_state_with_client(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    let mut source = FootSoldier::new(opponent_id);
    let source_id = *source.get_id();
    source.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(source));

    let mut bodyguard = RoyalBodyguard::new(player_id);
    let bodyguard_id = *bodyguard.get_id();
    bodyguard.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(bodyguard));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            if let ServerMessage::PickAction {
                player_id, actions, ..
            } = message
            {
                assert_eq!(actions[0], "Yes");
                client_tx
                    .send(ClientMessage::PickAction {
                        game_id,
                        player_id,
                        action_idx: 0,
                    })
                    .await
                    .unwrap();
            }
        }
    });

    state.queue_one(Effect::TakeDamage {
        card_id: avatar_id,
        from: source_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&bodyguard_id).get_damage_taken().unwrap(), 1);
}

#[tokio::test]
async fn test_seirawan_hydra_heals_nonlethal_damage_after_taking_it() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut source = FootSoldier::new(opponent_id);
    let source_id = *source.get_id();
    source.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(source));

    let mut hydra = SeirawanHydra::new(player_id);
    let hydra_id = *hydra.get_id();
    hydra.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(hydra));

    state.queue_one(Effect::TakeDamage {
        card_id: hydra_id,
        from: source_id,
        damage: Damage::basic(1),
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&hydra_id).get_damage_taken().unwrap(), 0);
}

#[tokio::test]
async fn test_effect_description_can_use_removed_card_lookup() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut attacker = ApprenticeWizard::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(attacker));

    let mut token = FootSoldier::new(player_id);
    let token_id = *token.get_id();
    token.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(token));

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
    state.add_card(Box::new(token));

    state.queue_one(Effect::BuryCard { card_id: token_id });
    drain_effects(&mut state).await;

    assert!(
        !state.contains_card(&token_id),
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
    state.add_card(Box::new(attacker));

    let mut defender = ApprenticeWizard::new(opponent_id);
    let defender_id = *defender.get_id();
    defender.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    defender.add_status(CardStatus::Disabled);
    state.add_card(Box::new(defender));

    state.queue_one(Effect::Fight {
        attacker_id,
        defender_id,
        defending_ids: vec![],
        damage_assignment: None,
        context: FightContext::Attack,
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
    state.add_card(Box::new(attacker));

    let mut defender_one = ApprenticeWizard::new(opponent_id);
    let defender_one_id = *defender_one.get_id();
    defender_one.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(defender_one));

    let mut defender_two = ApprenticeWizard::new(opponent_id);
    let defender_two_id = *defender_two.get_id();
    defender_two.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state.add_card(Box::new(defender_two));

    state.queue_one(Effect::Fight {
        attacker_id,
        defender_id: defender_one_id,
        defending_ids: vec![defender_one_id, defender_two_id],
        damage_assignment: Some(HashMap::from([(defender_one_id, 1), (defender_two_id, 2)])),
        context: FightContext::Attack,
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
async fn test_attack_action_declares_attack_only() {
    let attacker_zone = Zone::Location(Location::Square(1, Region::Surface));
    let attack_location = Zone::Location(Location::Square(2, Region::Surface));
    let (mut state, server_rx, client_tx) =
        make_state_with_client(vec![attacker_zone.clone(), attack_location.clone()]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut attacker = OgreGoons::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(attacker_zone.clone());
    state.add_card(Box::new(attacker));

    let mut attacked = ApprenticeWizard::new(opponent_id);
    let attacked_id = *attacked.get_id();
    attacked.set_zone(attack_location.clone());
    state.add_card(Box::new(attacked));

    let mut defender = OgreGoons::new(opponent_id);
    let defender_id = *defender.get_id();
    defender.set_zone(attack_location.clone());
    state.add_card(Box::new(defender));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickCard {
                    player_id, cards, ..
                } if cards.contains(&attacked_id) => {
                    client_tx
                        .send(ClientMessage::PickCard {
                            game_id,
                            player_id,
                            card_id: attacked_id,
                        })
                        .await
                        .unwrap();
                }
                ServerMessage::PickAction {
                    player_id, actions, ..
                } if actions == ["Yes", "No"] => {
                    client_tx
                        .send(ClientMessage::PickAction {
                            game_id,
                            player_id,
                            action_idx: 0,
                        })
                        .await
                        .unwrap();
                }
                ServerMessage::PickCards {
                    player_id, cards, ..
                } if cards.contains(&defender_id) => {
                    client_tx
                        .send(ClientMessage::PickCards {
                            game_id,
                            player_id,
                            card_ids: vec![defender_id],
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    let effects = UnitAction::Attack
        .on_select(&attacker_id, &player_id, &state)
        .await
        .expect("attack action should resolve prompts");

    assert!(
        matches!(
            effects.as_slice(),
            [Effect::DeclareAttack {
                attacker_id: actual_attacker_id,
                target_id,
            }] if *actual_attacker_id == attacker_id && *target_id == attacked_id
        ),
        "attack selection should only declare the attack; defender selection happens after attack triggers"
    );
}

#[tokio::test]
async fn test_multiple_defenders_fight_at_attacked_location() {
    let attacker_zone = Zone::Location(Location::Square(1, Region::Surface));
    let attack_location = Zone::Location(Location::Square(2, Region::Surface));
    let defender_one_start = Zone::Location(Location::Square(3, Region::Surface));
    let defender_two_start = Zone::Location(Location::Square(4, Region::Surface));
    let (mut state, _rx) = make_state(vec![
        attacker_zone.clone(),
        attack_location.clone(),
        defender_one_start.clone(),
        defender_two_start.clone(),
    ]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut attacker = OgreGoons::new(player_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(attacker_zone);
    state.add_card(Box::new(attacker));

    let mut attacked = ApprenticeWizard::new(opponent_id);
    let attacked_id = *attacked.get_id();
    attacked.set_zone(attack_location.clone());
    state.add_card(Box::new(attacked));

    let mut defender_one = OgreGoons::new(opponent_id);
    let defender_one_id = *defender_one.get_id();
    defender_one.set_zone(defender_one_start);
    state.add_card(Box::new(defender_one));

    let mut defender_two = OgreGoons::new(opponent_id);
    let defender_two_id = *defender_two.get_id();
    defender_two.set_zone(defender_two_start);
    state.add_card(Box::new(defender_two));

    state.queue_one(Effect::Fight {
        attacker_id,
        defender_id: attacked_id,
        defending_ids: vec![defender_one_id, defender_two_id],
        damage_assignment: Some(HashMap::from([(defender_one_id, 1), (defender_two_id, 2)])),
        context: FightContext::Attack,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&defender_one_id).get_zone(),
        &attack_location
    );
    assert_eq!(
        state.get_card(&defender_two_id).get_zone(),
        &attack_location
    );
    assert_eq!(
        state.get_card(&attacker_id).get_zone(),
        &Zone::Cemetery,
        "the defenders should strike back from the attacked location"
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
    state.add_card(Box::new(attacker));

    let mut first_striker = ApprenticeWizard::new(opponent_id);
    let first_striker_id = *first_striker.get_id();
    first_striker.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    first_striker
        .get_unit_base_mut()
        .unwrap()
        .abilities
        .push(Ability::FirstStrike);
    state.add_card(Box::new(first_striker));

    let mut other_defender = ApprenticeWizard::new(opponent_id);
    let other_defender_id = *other_defender.get_id();
    other_defender.set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state.add_card(Box::new(other_defender));

    state.queue_one(Effect::Fight {
        attacker_id,
        defender_id: other_defender_id,
        defending_ids: vec![first_striker_id, other_defender_id],
        damage_assignment: Some(HashMap::from([
            (first_striker_id, 0),
            (other_defender_id, 1),
        ])),
        context: FightContext::Attack,
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
    let opponent_id = state.players[1].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    let attacker_id = state.get_player_avatar_id(&opponent_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    let mut disabled_defender = FootSoldier::new(player_id);
    let disabled_defender_id = *disabled_defender.get_id();
    disabled_defender.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    disabled_defender.add_status(CardStatus::Disabled);
    state.add_card(Box::new(disabled_defender));

    let mut able_defender = FootSoldier::new(player_id);
    let able_defender_id = *able_defender.get_id();
    able_defender.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(able_defender));

    let defenders = state.get_defenders_for_attack(&attacker_id, &avatar_id);
    assert!(
        !defenders.contains(&disabled_defender_id),
        "disabled units should not be valid defenders"
    );
    assert!(
        defenders.contains(&able_defender_id),
        "able nearby units should remain valid defenders"
    );

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
            Location::Square(2, Region::Surface),
            Location::Square(3, Region::Surface),
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
async fn test_silenced_bridge_troll_hook_does_not_trigger() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut bridge_troll = BridgeTroll::new(player_id);
    let bridge_troll_id = *bridge_troll.get_id();
    bridge_troll.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    bridge_troll.add_status(CardStatus::Silenced);
    state.add_card(Box::new(bridge_troll));

    let mut attacker = ApprenticeWizard::new(opponent_id);
    let attacker_id = *attacker.get_id();
    attacker.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(attacker));

    *state.get_player_mana_mut(&opponent_id) = 5;

    state.queue_one(Effect::DeclareAttack {
        attacker_id,
        target_id: bridge_troll_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        *state.get_player_mana_mut(&opponent_id),
        5,
        "silenced Bridge Troll should not drain the attacker's mana"
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
async fn test_adjust_avatar_life_changes_life_without_damage_death_blow() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();

    state.queue_one(Effect::AdjustAvatarLife {
        player_id,
        amount: -5,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&avatar_id).get_damage_taken().unwrap(),
        5,
        "losing life should lower avatar life without going through TakeDamage"
    );

    state.queue_one(Effect::AdjustAvatarLife {
        player_id,
        amount: 2,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&avatar_id).get_damage_taken().unwrap(),
        3,
        "gaining life should increase avatar life up to its maximum"
    );

    state.queue_one(Effect::AdjustAvatarLife {
        player_id,
        amount: -20,
    });
    drain_effects(&mut state).await;

    assert!(
        state
            .get_card(&avatar_id)
            .get_avatar_base()
            .unwrap()
            .deaths_door,
        "life loss to 0 should put an avatar at Death's Door"
    );

    state
        .get_card_mut(&avatar_id)
        .get_avatar_base_mut()
        .unwrap()
        .can_die = true;

    state.queue_one(Effect::AdjustAvatarLife {
        player_id,
        amount: -1,
    });
    drain_effects(&mut state).await;

    assert!(
        !state.eliminated_players.contains(&player_id),
        "life loss after Death's Door should not be treated as direct damage death blow"
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
    state.add_card(Box::new(site));

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
    let (mut state, server_rx, client_tx) = make_state_with_client(vec![]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    let mut vaults = VaultsOfZul::new(player_id);
    let vaults_id = *vaults.get_id();
    vaults.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(vaults));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(3, Region::Surface).with_region(Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Location::Square(2, Region::Surface),
            Location::Square(3, Region::Surface),
        ]),
    });
    drain_effects(&mut state).await;

    assert!(
        !state.eliminated_players.contains(&player_id),
        "Vaults of Zul should not trigger when an Avatar only enters it mid-movement"
    );

    state
        .get_card_mut(&vaults_id)
        .set_zone(Zone::Location(Location::Square(3, Region::Surface)));
    state
        .get_card_mut(&avatar_id)
        .set_zone(Zone::Location(Location::Square(1, Region::Surface)));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            if let ServerMessage::PickAction { player_id, .. } = message {
                client_tx
                    .send(ClientMessage::PickAction {
                        game_id,
                        player_id,
                        action_idx: 0,
                    })
                    .await
                    .unwrap();
            }
        }
    });

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: avatar_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(3, Region::Surface).with_region(Region::Surface),
        ),
        tap: false,
        through_path: Some(vec![
            Location::Square(2, Region::Surface),
            Location::Square(3, Region::Surface),
        ]),
    });
    drain_effects(&mut state).await;

    assert!(
        state.eliminated_players.contains(&player_id),
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
    state.add_card(Box::new(pit));

    let ogre = OgreGoons::new(player_id);
    let ogre_id = *ogre.get_id();
    state.add_card(Box::new(ogre));

    state.queue_one(Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: ogre_id,
            from_zone: Zone::Cemetery,
            to_location: Location::Square(1, Region::Surface),
        }],
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&ogre_id).get_zone(),
        &Zone::Cemetery,
        "Bottomless Pit should trigger when a minion is summoned into it"
    );
}

#[tokio::test]
async fn test_planar_gate_grants_voidwalk_only_while_minion_is_there() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;

    let mut gate = PlanarGate::new(player_id);
    let gate_id = *gate.get_id();
    gate.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(gate));
    state
        .add_passive_ongoing_effects_for_source(&gate_id)
        .await
        .unwrap();

    let mut destination = AridDesert::new(player_id);
    let destination_id = *destination.get_id();
    destination.set_zone(Zone::Location(Location::Square(2, Region::Surface)));
    state.add_card(Box::new(destination));

    let mut foot_soldier = FootSoldier::new(player_id);
    let foot_soldier_id = *foot_soldier.get_id();
    foot_soldier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(foot_soldier));

    assert!(
        state
            .get_card(&foot_soldier_id)
            .has_ability(&state, &Ability::Voidwalk),
        "Planar Gate should grant Voidwalk to minions at the site"
    );

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: foot_soldier_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(Region::Surface),
        ),
        tap: false,
        through_path: None,
    });
    drain_effects(&mut state).await;

    assert!(
        !state
            .get_card(&foot_soldier_id)
            .has_ability(&state, &Ability::Voidwalk),
        "Planar Gate should stop granting Voidwalk once the minion leaves"
    );
}

#[tokio::test]
async fn test_wall_of_fire_damages_unit_crossing_its_border() {
    let (mut state, _rx) = make_state(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
    ]);
    let player_id = state.players[0].id;

    let mut wall = WallOfFire::new(player_id);
    let wall_id = *wall.get_id();
    wall.set_zone(Zone::Location(Location::Intersection(
        vec![1, 2, 6, 7],
        Region::Surface,
    )));
    state.add_card(Box::new(wall));

    let mut foot_soldier = FootSoldier::new(player_id);
    let foot_soldier_id = *foot_soldier.get_id();
    foot_soldier.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(foot_soldier));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: foot_soldier_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(Region::Surface),
        ),
        tap: false,
        through_path: None,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&foot_soldier_id).get_zone(),
        &Zone::Cemetery,
        "Wall of Fire should damage a unit that crosses its border"
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
    state.add_card(Box::new(assassin));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: assassin_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(2, Region::Surface).with_region(Region::Surface),
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
        to: LocationQuery::from_location(
            Location::Square(3, Region::Surface).with_region(Region::Surface),
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
    state.add_card(Box::new(assassin));

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
async fn test_minion_without_burrowing_dies_underground() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    apprentice_wizard.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(apprentice_wizard));

    state.queue_one(Effect::MoveCard {
        player_id,
        card_id: apprentice_wizard_id,
        from: Location::Square(1, Region::Surface),
        to: LocationQuery::from_location(
            Location::Square(1, Region::Surface).with_region(Region::Underground),
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
    state.add_card(Box::new(apprentice_wizard));

    state.queue_one(Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: apprentice_wizard_id,
            from_zone: Zone::Hand,
            to_location: Location::Square(1, Region::Void),
        }],
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
    state.add_card(Box::new(apprentice_wizard));

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
    state.add_card(Box::new(spring_river));

    let mut apprentice_wizard = ApprenticeWizard::new(player_id);
    let apprentice_wizard_id = *apprentice_wizard.get_id();
    apprentice_wizard.add_ability(Ability::Submerge);
    apprentice_wizard.set_zone(Zone::Location(Location::Square(1, Region::Underwater)));
    state.add_card(Box::new(apprentice_wizard));

    let mut drought = Drought::new(player_id);
    let drought_id = *drought.get_id();
    drought.set_zone(Zone::Hand);
    state.add_card(Box::new(drought));

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
    state.add_card(Box::new(spring_river));

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
    state.add_card(Box::new(spring_river));

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
    state.add_card(Box::new(spring_river));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: spring_river_id,
        location: Location::Square(1, Region::Surface),
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
async fn test_playing_site_under_flood_does_not_recurse() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(2, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut flood = Flood::new(player_id);
    let flood_id = *flood.get_id();
    flood.set_zone(Zone::Location(Location::Intersection(
        vec![1, 2, 6, 7],
        Region::Surface,
    )));
    state.add_card(Box::new(flood));
    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let spring_river = SpringRiver::new(player_id);
    let spring_river_id = *spring_river.get_id();
    state.add_card(Box::new(spring_river));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: spring_river_id,
        location: Location::Square(1, Region::Surface),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let site = state
        .get_card(&spring_river_id)
        .get_resource_provider()
        .expect("Spring River should be a resource provider");
    assert_eq!(site.provided_affinity(&state).unwrap().water, 1);
}

#[tokio::test]
async fn test_temporary_effect_grants_ability() {
    let (mut state, _rx) = make_state(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let unit = FootSoldier::new(player_id);
    let unit_id = *unit.get_id();
    state.add_card(Box::new(unit.clone()));
    state
        .temporary_effects_mut()
        .push(TemporaryEffect::GrantAbility {
            ability: Ability::FirstStrike,
            affected_cards: CardQuery::from_id(unit_id).including_not_in_play(),
            expires_on_effect: EffectQuery::TurnEnd { player_id: None },
        });

    let has_first_strike = unit.has_ability(&state, &Ability::FirstStrike);
    assert!(has_first_strike);
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
    let minion = TestHookSource::new(player_id, HookSourceZones::InPlay);
    let card_id = *minion.get_id();
    state.add_card(Box::new(minion));

    state.deferred_effects_mut().push(DeferredEffect {
        hook_id: TEST_HOOK_SOURCE_ID,
        card_id,
        trigger_on_effect: EffectQuery::DrawCard { player_id: None },
        expires_on_effect: None,
        trigger_times: Some(1),
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
    let minion = TestHookSource::new(player_id, HookSourceZones::InPlay);
    let card_id = *minion.get_id();
    state.add_card(Box::new(minion));

    state.deferred_effects_mut().push(DeferredEffect {
        hook_id: TEST_HOOK_SOURCE_ID,
        card_id,
        trigger_on_effect: EffectQuery::DrawCard { player_id: None },
        expires_on_effect: None,
        trigger_times: None,
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
    let minion = TestHookSource::new(player_id, HookSourceZones::InPlay);
    let card_id = *minion.get_id();
    state.add_card(Box::new(minion));

    state.deferred_effects_mut().push(DeferredEffect {
        hook_id: TEST_HOOK_SOURCE_ID,
        card_id,
        trigger_on_effect: EffectQuery::TurnStart { player_id: None },
        expires_on_effect: Some(EffectQuery::DrawCard { player_id: None }),
        trigger_times: None,
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
        .cards_in_play()
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
    state.add_card(Box::new(minion));

    Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: id,
            from_zone: Zone::Hand,
            to_location: Location::Square(1, Region::Surface),
        }],
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
    state.add_card(Box::new(minion));

    Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: id,
            from_zone: Zone::Hand,
            to_location: Location::Square(1, Region::Surface),
        }],
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
    state.add_card(Box::new(minion));

    Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: id,
            from_zone: Zone::Hand,
            to_location: Location::Square(1, Region::Surface),
        }],
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
    let location = Zone::Location(Location::Square(1, Region::Surface));
    let (mut state, _rx) = make_state(vec![location.clone()]);
    let player_id = state.players[0].id;

    let wizard = ApprenticeWizard::new(player_id);
    let id = *wizard.get_id();
    state.add_card(Box::new(wizard));

    Effect::SummonCards {
        summoned_cards: vec![SummonCard {
            player_id,
            card_id: id,
            from_zone: Zone::Hand,
            to_location: location
                .clone()
                .location()
                .cloned()
                .expect("test zone must be a location"),
        }],
    }
    .apply(&mut state)
    .await
    .unwrap();

    let has_genesis_trigger = state.effects.iter().any(|e| {
        matches!(
            *e,
            Effect::TriggerGenesis {
                card_id
            } if card_id == id
        )
    });
    assert!(
        has_genesis_trigger,
        "SummonCards should queue a genesis trigger for ApprenticeWizard"
    );

    drain_effects(&mut state).await;
    assert_eq!(state.get_card(&id).get_zone(), &location);
}

// TODO: Genesis does not trigger if the card is disabled by an ongoing effect. Not sure in what
// situation it would.
// #[tokio::test]
// async fn test_genesis_triggers_even_if_source_is_disabled() {
//     let zone = Zone::Location(Location::Square(1, Region::Surface));
//     let (mut state, _rx) = make_state(vec![zone.clone()]);
//     let player_id = state.players[0].id;
//
//     let mut wizard = ApprenticeWizard::new(player_id);
//     let wizard_id = *wizard.get_id();
//     wizard.set_zone(zone);
//     wizard.add_status(CardStatus::Disabled);
//     state.add_card(Box::new(wizard));
//
//     state.queue_one(Effect::TriggerGenesis { card_id: wizard_id });
//     drain_effects(&mut state).await;
//
//     assert!(
//         state.eliminated_players.contains(&player_id),
//         "disabled Genesis sources should still resolve their enters-the-realm trigger"
//     );
// }

#[tokio::test]
async fn test_genesis_is_ignored_if_source_left_realm_before_resolution() {
    let location = Zone::Location(Location::Square(1, Region::Surface));
    let (mut state, _rx) = make_state(vec![location.clone()]);
    let player_id = state.players[0].id;

    let mut wizard = ApprenticeWizard::new(player_id);
    let wizard_id = *wizard.get_id();
    wizard.set_zone(location);
    state.add_card(Box::new(wizard));

    state.queue(vec![
        Effect::TriggerGenesis { card_id: wizard_id },
        Effect::BanishCard { card_id: wizard_id },
    ]);
    drain_effects(&mut state).await;

    assert!(
        !state.eliminated_players.contains(&player_id),
        "Genesis should be ignored after its source has left the realm"
    );
    assert_eq!(state.get_card(&wizard_id).get_zone(), &Zone::Banish);
}

#[tokio::test]
async fn test_deathrite_resolves_before_source_enters_cemetery() {
    let location = Zone::Location(Location::Square(1, Region::Surface));
    let (mut state, _rx) = make_state(vec![location.clone()]);
    let player_id = state.players[0].id;

    let mut scarabs = SacredScarabs::new(player_id);
    let scarabs_id = *scarabs.get_id();
    scarabs.set_zone(location);
    state.add_card(Box::new(scarabs));

    state.queue_one(Effect::BuryCard {
        card_id: scarabs_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&scarabs_id).get_zone(), &Zone::Cemetery);
    assert_eq!(
        state
            .get_card(&scarabs_id)
            .get_damage_taken()
            .unwrap_or_default(),
        3,
        "Sacred Scarabs should still occupy its death zone while Deathrite resolves"
    );
}

#[tokio::test]
async fn test_disabled_deathrite_source_does_not_resolve() {
    let location = Zone::Location(Location::Square(1, Region::Surface));
    let (mut state, _rx) = make_state(vec![location.clone()]);
    let player_id = state.players[0].id;

    let mut scarabs = SacredScarabs::new(player_id);
    let scarabs_id = *scarabs.get_id();
    scarabs.set_zone(location);
    scarabs.add_status(CardStatus::Disabled);
    state.add_card(Box::new(scarabs));

    state.queue_one(Effect::BuryCard {
        card_id: scarabs_id,
    });
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&scarabs_id).get_zone(), &Zone::Cemetery);
    assert_eq!(
        state
            .get_card(&scarabs_id)
            .get_damage_taken()
            .unwrap_or_default(),
        0,
        "disabled Deathrite sources should not resolve"
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
        location: Location::Square(1, Region::Surface),
        spellcaster: avatar_one_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
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
    state.add_card(Box::new(ogre));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        location: Location::Square(1, Region::Surface),
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
async fn test_play_card_burrowing_minion_can_enter_underground() {
    let (mut state, _rx) = make_state(vec![]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 3;

    let mut site = SimpleVillage::new(player_id);
    site.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(site));

    let mut spider = RootSpider::new(player_id);
    let spider_id = *spider.get_id();
    spider.set_zone(Zone::Hand);
    state.add_card(Box::new(spider));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: spider_id,
        location: Location::Square(1, Region::Underground),
        spellcaster: avatar_id,
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&spider_id).get_zone(),
        &Zone::Location(Location::Square(1, Region::Underground)),
        "burrowing minion should survive underground below a land site"
    );
}

#[tokio::test]
async fn test_enchantress_triggers_when_controlled_spellcaster_plays_minion() {
    let location = Location::Square(1, Region::Surface);
    let (mut state, server_rx, client_tx) = make_state_with_client(vec![location.clone().into()]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 1;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(location.clone().into());
    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let mut aura = Silence::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(location.clone().into());
    state.add_card(Box::new(aura));

    let mut caster = ApprenticeWizard::new(player_id);
    let caster_id = *caster.get_id();
    caster.set_zone(location.clone().into());
    state.add_card(Box::new(caster));

    let mut spell = PitVipers::new(player_id);
    let spell_id = *spell.get_id();
    spell.set_zone(Zone::Hand);
    state.add_card(Box::new(spell));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickAction {
                    player_id, actions, ..
                } => {
                    assert_eq!(actions[0], "Yes");
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
                } if cards.contains(&aura_id) => {
                    client_tx
                        .send(ClientMessage::PickCard {
                            game_id,
                            player_id,
                            card_id: aura_id,
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    state.queue_one(Effect::PlayCard {
        player_id,
        card_id: spell_id,
        location: location.clone(),
        spellcaster: caster_id,
    });
    drain_effects(&mut state).await;

    assert!(
        state.is_minion_card(&aura_id),
        "Enchantress should trigger from a minion spell played by another controlled spellcaster"
    );
    assert_eq!(
        state.get_card(&aura_id).get_location(),
        &location,
        "the animated aura should remain in the realm after location survival checks"
    );
}

#[tokio::test]
async fn test_enchantress_triggers_when_enchantress_plays_minion() {
    let location = Location::Square(1, Region::Surface);
    let intersection = Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface));
    let (mut state, server_rx, client_tx) = make_state_with_client(vec![
        location.clone().into(),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(6, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
    ]);
    let game_id = state.game_id;
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 1;
    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state
        .get_card_mut(&avatar_id)
        .set_zone(location.clone().into());
    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let mut aura = Silence::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(intersection.clone());
    state.add_card(Box::new(aura));
    assert!(
        CardQuery::new()
            .auras()
            .in_play()
            .all(&state)
            .contains(&aura_id),
        "CardQuery::in_play should include auras on intersections"
    );

    let mut spell = PitVipers::new(player_id);
    let spell_id = *spell.get_id();
    spell.set_zone(Zone::Hand);
    state.add_card(Box::new(spell));

    tokio::spawn(async move {
        while let Ok(message) = server_rx.recv().await {
            match message {
                ServerMessage::PickAction {
                    player_id, actions, ..
                } => {
                    assert_eq!(actions[0], "Yes");
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
                } if cards.contains(&aura_id) => {
                    client_tx
                        .send(ClientMessage::PickCard {
                            game_id,
                            player_id,
                            card_id: aura_id,
                        })
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    });

    state.queue_one(Effect::PlayCard {
        player_id,
        card_id: spell_id,
        location,
        spellcaster: avatar_id,
    });
    drain_effects(&mut state).await;

    assert!(
        state.is_minion_card(&aura_id),
        "Enchantress should trigger when she is the implicit spellcaster"
    );
    assert_eq!(
        state.get_card(&aura_id).get_zone(),
        &intersection,
        "the animated aura should remain in the realm after location survival checks"
    );
}

#[tokio::test]
async fn test_enchantress_does_not_see_aura_spell_before_it_enters() {
    let location = Location::Intersection(vec![1, 2, 6, 7], Region::Surface);
    let (mut state, _server_rx, _client_tx) =
        make_state_with_client(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;
    *state.get_player_mana_mut(&player_id) = 1;
    state.reconcile_ongoing_effects_for_test().await.unwrap();

    let mut aura = Sandstorm::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(Zone::Hand);
    state.add_card(Box::new(aura));

    let avatar_id = state.get_player_avatar_id(&player_id).unwrap();
    state.queue_one(Effect::PlayCard {
        player_id,
        card_id: aura_id,
        location,
        spellcaster: avatar_id,
    });

    tokio::time::timeout(
        std::time::Duration::from_millis(100),
        drain_effects(&mut state),
    )
    .await
    .expect("Enchantress should not prompt for the aura spell before it enters");

    assert!(
        CardQuery::new()
            .auras()
            .in_play()
            .all(&state)
            .contains(&aura_id),
        "the aura spell should resolve into play"
    );
    assert!(
        !state.is_minion_card(&aura_id),
        "Enchantress should not animate the aura spell that caused the trigger"
    );
}

#[tokio::test]
async fn test_animated_intersection_aura_gets_unit_actions_and_stays_on_intersection_attack() {
    let intersection = Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface));
    let (mut state, _server_rx, _client_tx) = make_state_with_client(vec![
        Zone::Location(Location::Square(1, Region::Surface)),
        Zone::Location(Location::Square(2, Region::Surface)),
        Zone::Location(Location::Square(6, Region::Surface)),
        Zone::Location(Location::Square(7, Region::Surface)),
        Zone::Location(Location::Square(3, Region::Surface)),
        Zone::Location(Location::Square(8, Region::Surface)),
    ]);
    let player_id = state.players[0].id;
    let opponent_id = state.players[1].id;

    let mut aura = Silence::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(intersection.clone());
    state.add_card(Box::new(aura));

    let mut target = FootSoldier::new(opponent_id);
    let target_id = *target.get_id();
    target.set_zone(Zone::Location(Location::Square(1, Region::Surface)));
    state.add_card(Box::new(target));

    Effect::Animate {
        card_id: aura_id,
        unit_base: UnitBase {
            power: 0,
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
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&aura_id).get_zone(),
        &intersection,
        "animated intersection auras over sites should survive location checks"
    );

    let action_names = state
        .get_card(&aura_id)
        .get_activated_abilities(&state)
        .unwrap()
        .into_iter()
        .map(|ability| ability.get_name())
        .collect::<Vec<_>>();
    assert!(action_names.contains(&"Attack".to_string()));
    assert!(action_names.contains(&"Move".to_string()));

    let move_zones = state
        .get_card(&aura_id)
        .get_valid_move_locations(&state)
        .await
        .unwrap();
    assert!(
        move_zones
            .iter()
            .all(|zone| matches!(zone, Location::Intersection(_, _))),
        "animated intersection auras should move from intersection to intersection"
    );
    assert!(
        !move_zones.contains(&Location::Square(1, Region::Surface)),
        "animated intersection auras should not move to a constituent square"
    );

    let attack_targets = state
        .get_card(&aura_id)
        .get_valid_attack_targets(&state, false);
    assert!(
        attack_targets.contains(&target_id),
        "animated intersection auras should be able to attack units in occupied squares"
    );

    state.queue_one(Effect::Fight {
        attacker_id: aura_id,
        defender_id: target_id,
        defending_ids: vec![],
        damage_assignment: None,
        context: FightContext::Attack,
    });
    drain_effects(&mut state).await;

    assert_eq!(
        state.get_card(&aura_id).get_zone(),
        &intersection,
        "attacking a unit in an occupied square should not move the intersection aura"
    );
}

#[tokio::test]
async fn test_animated_intersection_aura_in_void_is_banished_without_voidwalk() {
    let intersection = Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface));
    let (mut state, _server_rx, _client_tx) =
        make_state_with_client(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut aura = Silence::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(intersection);
    state.add_card(Box::new(aura));

    Effect::Animate {
        card_id: aura_id,
        unit_base: UnitBase {
            power: 1,
            toughness: 1,
            ..Default::default()
        },
        expires_on_effect: EffectQuery::TurnStart {
            player_id: Some(player_id),
        },
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    assert_eq!(state.get_card(&aura_id).get_zone(), &Zone::Banish);
}

#[tokio::test]
async fn test_animated_intersection_aura_query_does_not_recurse_with_unit_modifier() {
    let intersection = Zone::Location(Location::Intersection(vec![1, 2, 6, 7], Region::Surface));
    let (mut state, _server_rx, _client_tx) =
        make_state_with_client(vec![Zone::Location(Location::Square(1, Region::Surface))]);
    let player_id = state.players[0].id;

    let mut aura = Silence::new(player_id);
    let aura_id = *aura.get_id();
    aura.set_zone(intersection);
    state.add_card(Box::new(aura));

    Effect::Animate {
        card_id: aura_id,
        unit_base: UnitBase {
            power: 1,
            toughness: 1,
            ..Default::default()
        },
        expires_on_effect: EffectQuery::TurnStart {
            player_id: Some(player_id),
        },
    }
    .apply(&mut state)
    .await
    .unwrap();

    state
        .temporary_effects_mut()
        .push(TemporaryEffect::GrantAbility {
            ability: Ability::Airborne,
            affected_cards: CardQuery::new().units().in_play(),
            expires_on_effect: EffectQuery::TurnStart {
                player_id: Some(player_id),
            },
        });

    let units = CardQuery::new().units().in_play().all(&state);
    assert!(units.contains(&aura_id));
    assert!(
        state
            .get_card(&aura_id)
            .has_ability(&state, &Ability::Airborne)
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
    state.add_card(Box::new(ogre));

    let avatar_id = state
        .get_player_avatar_id(&player_id)
        .expect("avatar id to be some");
    Effect::PlayCard {
        player_id,
        card_id: ogre_id,
        location: Location::Square(1, Region::Surface),
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
        location: Location::Square(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldiers = CardQuery::new()
        .named(FootSoldier::NAME.to_string())
        .all(&state);
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
        location: Location::Square(1, Region::Surface),
    }
    .apply(&mut state)
    .await
    .unwrap();
    drain_effects(&mut state).await;

    let soldiers = CardQuery::new()
        .named(FootSoldier::NAME.to_string())
        .all(&state);
    assert!(
        soldier.has_status(&state, &CardStatus::SummoningSickness),
        "summoned unit token should have SummoningSickness"
    );
}
