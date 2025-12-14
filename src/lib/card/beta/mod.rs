pub mod clamor_of_harpies;
pub use clamor_of_harpies::*;
pub mod flamecaller;
pub use flamecaller::*;
pub mod arid_desert;
pub use arid_desert::*;
pub mod pit_vipers;
pub use pit_vipers::*;
pub mod raal_dromedary;
pub use raal_dromedary::*;
pub mod lava_salamander;
pub use lava_salamander::*;
pub mod sacred_scarabs;
pub use sacred_scarabs::*;
pub mod wayfaring_pilgrim;
pub use wayfaring_pilgrim::*;

use crate::{card::Card, game::PlayerId};

pub fn from_beta_name(name: &str, player_id: PlayerId) -> Option<Box<dyn Card>> {
    match name {
        Flamecaller::NAME => Some(Box::new(Flamecaller::new(player_id))),
        ClamorOfHarpies::NAME => Some(Box::new(ClamorOfHarpies::new(player_id))),
        AridDesert::NAME => Some(Box::new(AridDesert::new(player_id))),
        PitVipers::NAME => Some(Box::new(PitVipers::new(player_id))),
        RaalDromedary::NAME => Some(Box::new(RaalDromedary::new(player_id))),
        LavaSalamander::NAME => Some(Box::new(LavaSalamander::new(player_id))),
        SacredScarabs::NAME => Some(Box::new(SacredScarabs::new(player_id))),
        WayfaringPilgrim::NAME => Some(Box::new(WayfaringPilgrim::new(player_id))),
        _ => None,
    }
}
