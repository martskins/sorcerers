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
    AridDesert,
    SpringRiver,
    Aqueduct,
    AstralAlcazar,
    Cornerstone,
    RemoteDesert,
    RedDesert,
    ShiftingSands,
    Vesuvius
}
