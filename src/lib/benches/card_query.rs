use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::seq::IndexedRandom;
use sorcerers::{
    card::{ALL_CARDS, ApprenticeWizard, Card, PoisonousDagger, Region},
    query::CardQuery,
    state::State,
    zone::Zone,
};

fn setup_state_with_cards(num_cards: usize) -> State {
    // We expect the 'benchmark' feature to be enabled for this to work
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    // Fill state with many cards
    ALL_CARDS
        .choose_multiple(&mut rand::rng(), num_cards)
        .for_each(|(_, constructor)| {
            let mut card = constructor(player_id);
            card.set_zone(Zone::Location(
                (card.get_id().as_u128() % 25) as u8,
                Region::Surface,
            ));
            state.cards.insert(*card.get_id(), card);
        });

    state
}

fn bench_card_query(c: &mut Criterion) {
    let mut state = setup_state_with_cards(180);
    let player_id = state.players[0].id;
    let mut card = ApprenticeWizard::new(player_id);
    card.set_zone(Zone::Location(8, Region::Surface));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut group = c.benchmark_group("CardQuery");
    group.bench_function("Zone + Untapped", |b| {
        b.iter(|| {
            let query = CardQuery::new()
                .in_zone(&Zone::Location(10, Region::Surface))
                .untapped();
            let _ = black_box(query.all(&state));
        })
    });

    group.bench_function("Name Contains", |b| {
        b.iter(|| {
            let query = CardQuery::new().card_name_contains("Wizard");
            let _ = black_box(query.all(&state));
        })
    });

    group.bench_function("Within Range Of", |b| {
        b.iter(|| {
            let query = CardQuery::new().within_range_of(card.get_id());
            let _ = black_box(query.all(&state));
        })
    });

    group.finish();
}

fn bench_card_query_manual(c: &mut Criterion) {
    let mut state = setup_state_with_cards(180);
    let player_id = state.players[0].id;

    let mut card = ApprenticeWizard::new(player_id);
    let card_id = *card.get_id();
    card.set_zone(Zone::Location(8, Region::Surface));
    state.cards.insert(*card.get_id(), Box::new(card.clone()));

    let mut dagger = PoisonousDagger::new(player_id);
    dagger.set_zone(Zone::Location(8, Region::Surface));
    dagger.set_bearer_id(Some(*card.get_id()));
    state
        .cards
        .insert(*dagger.get_id(), Box::new(dagger.clone()));

    let mut group = c.benchmark_group("Card Querying");
    group.bench_function("Manual Query", |b| {
        b.iter(|| {
            let borne_cards: Vec<uuid::Uuid> = state
                .cards
                .values()
                .filter(|c| c.get_zone().is_in_play())
                .filter_map(|c| {
                    c.get_bearer_id()
                        .ok()
                        .flatten()
                        .filter(|bearer_id| *bearer_id == card_id)
                        .map(|_| *c.get_id())
                })
                .collect();
            assert_eq!(borne_cards.len(), 1);
        });
    });

    group.bench_function("CardQuery", |b| {
        b.iter(|| {
            let borne_cards = CardQuery::new().carried_by(&card_id).all(&state);
            assert_eq!(borne_cards.len(), 1);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_card_query, bench_card_query_manual);
criterion_main!(benches);
