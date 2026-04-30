use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sorcerers::{
    card::{ApprenticeWizard, Card, Zone},
    state::{CardQuery, State},
};

fn bench_card_query(c: &mut Criterion) {
    let num_cards: usize = 500;
    // We expect the 'benchmark' feature to be enabled for this to work
    let mut state = State::new_mock_state(vec![]);
    let player_id = state.players[0].id;

    // Fill state with many cards
    for i in 0..num_cards {
        let mut card = ApprenticeWizard::new(player_id);
        card.set_zone(Zone::Realm((i % 25) as u8));
        state.cards.push(Box::new(card));
    }

    let mut group = c.benchmark_group("CardQuery");

    group.bench_function("Zone + Untapped", |b| {
        b.iter(|| {
            let query = CardQuery::new().in_zone(&Zone::Realm(10)).untapped();
            let _ = black_box(query.all(&state));
        })
    });

    group.bench_function("Name Contains", |b| {
        b.iter(|| {
            let query = CardQuery::new().card_name_contains("Wizard");
            let _ = black_box(query.all(&state));
        })
    });

    group.finish();
}

criterion_group!(benches, bench_card_query);
criterion_main!(benches);
