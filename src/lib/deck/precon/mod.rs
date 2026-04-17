use crate::{
    card::Card,
    deck::{Deck, precon},
    game::PlayerId,
};
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};

pub mod beta;

#[distributed_slice]
pub static ALL_PRECONS: [(
    &'static PreconDeck,
    fn(&PlayerId) -> (Deck, Vec<Box<dyn Card>>),
)];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreconDeck {
    BetaFire,
    BetaAir,
    BetaEarth,
    BetaWater,
}

impl PreconDeck {
    pub fn name(&self) -> &'static str {
        match self {
            PreconDeck::BetaFire => "Beta - Fire",
            PreconDeck::BetaAir => "Beta - Air",
            PreconDeck::BetaEarth => "Beta - Earth",
            PreconDeck::BetaWater => "Beta - Water",
        }
    }

    pub fn build(&self, player_id: &PlayerId) -> (Deck, Vec<Box<dyn Card>>) {
        match self {
            PreconDeck::BetaFire => precon::beta::fire(player_id),
            PreconDeck::BetaAir => precon::beta::air(player_id),
            PreconDeck::BetaEarth => precon::beta::earth(player_id),
            PreconDeck::BetaWater => precon::beta::water(player_id),
        }
    }
}
