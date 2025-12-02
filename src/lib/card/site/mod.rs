pub mod vesuvius;
pub use vesuvius::Vesuvius;
pub mod shifting_sands;
pub use shifting_sands::ShiftingSands;
pub mod remote_desert;
pub use remote_desert::RemoteDesert;
pub mod red_desert;
pub use red_desert::RedDesert;
pub mod cornerstone;
pub use cornerstone::Cornerstone;
pub mod aqueduct;
pub use aqueduct::Aqueduct;
pub mod arid_desert;
pub use arid_desert::AridDesert;
pub mod astral_alcazar;
pub use astral_alcazar::AstralAlcazar;
pub mod spring_river;
pub use spring_river::SpringRiver;

mod site;
mod util;

use crate::{
    card::{CardBase, CardZone, Edition},
    networking::Thresholds,
    sites,
};
use serde::{Deserialize, Serialize};
pub use site::*;

#[rustfmt::skip]
sites! {
    AridDesert, "Arid Desert",
    SpringRiver, "Spring River",
    Aqueduct, "Aqueduct",
    AstralAlcazar, "Astral Alcazar",
    Cornerstone, "Cornerstone",
    RemoteDesert, "Remote Desert",
    RedDesert, "Red Desert",
    ShiftingSands, "Shifting Sands",
    Vesuvius, "Vesuvius"
}
