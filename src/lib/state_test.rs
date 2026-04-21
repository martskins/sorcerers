use crate::{
    card::{
        Card, CauldronCrones, DonnybrookInn, HeadlessHaunt, KiteArcher, NimbusJinn, RimlandNomads,
        Zone,
    },
    game::Thresholds,
    query::EffectQuery,
    state::{CardQuery, State, TemporaryEffect},
};

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
