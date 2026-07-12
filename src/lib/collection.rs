use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedCard {
    pub name: String,
    pub count: u8,
    pub is_foil: bool,
}
