pub mod beta;
pub mod site;
mod util;

use super::Thresholds;
use crate::{
    card::{CardBase, CardZone, Edition},
    sites,
};
use beta::*;
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
