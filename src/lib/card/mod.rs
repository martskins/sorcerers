pub mod beta;
pub mod card;
pub mod rubble;
pub use beta::*;
pub use card::*;
pub use rubble::*;

use crate::{effect::Effect, game::PlayerId};
use linkme::distributed_slice;
use std::{collections::HashMap, sync::LazyLock};

fn place_rubble(controller_id: &PlayerId, zone: &Zone) -> Vec<Effect> {
    let rubble = Rubble::new(controller_id.clone());
    let rubble_id = rubble.get_id().clone();

    vec![
        Effect::AddCard { card: Box::new(rubble) },
        Effect::play_card(controller_id, &rubble_id, zone),
    ]
}

#[distributed_slice]
pub static ALL_CARDS: [(&'static str, fn(PlayerId) -> Box<dyn Card>)];

pub static CARD_CONSTRUCTORS: LazyLock<HashMap<&'static str, fn(PlayerId) -> Box<dyn Card>>> = LazyLock::new(|| {
    let mut constructors = HashMap::new();
    for (name, constructor) in ALL_CARDS {
        constructors.insert(*name, *constructor);
    }
    constructors
});
