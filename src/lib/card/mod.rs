pub mod beta;
pub mod card;
pub mod foot_soldier;
pub mod frog;
pub mod rubble;
pub use beta::*;
pub use card::*;
pub use foot_soldier::*;
pub use frog::*;
pub use rubble::*;

use crate::{deck::Deck, game::PlayerId, networking::message::PreconDeck};
use linkme::distributed_slice;
use std::{collections::HashMap, sync::LazyLock};

#[distributed_slice]
pub static ALL_CARDS: [(&'static str, fn(PlayerId) -> Box<dyn Card>)];

#[distributed_slice]
pub static ALL_PRECONS: [(&'static PreconDeck, fn(&PlayerId) -> (Deck, Vec<Box<dyn Card>>))];

pub static CARD_CONSTRUCTORS: LazyLock<HashMap<&'static str, fn(PlayerId) -> Box<dyn Card>>> = LazyLock::new(|| {
    let mut constructors = HashMap::new();
    for (name, constructor) in ALL_CARDS {
        constructors.insert(*name, *constructor);
    }
    constructors
});
