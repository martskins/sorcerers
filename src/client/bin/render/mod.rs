use macroquad::{math::Rect, texture::Texture2D};
use sorcerers::{
    card::{Modifier, Zone},
    game::PlayerId,
};

#[derive(Debug, Clone)]
pub struct CardRect {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub zone: Zone,
    pub tapped: bool,
    pub image: Texture2D,
    pub rect: Rect,
    pub is_hovered: bool,
    pub is_selected: bool,
    pub modifiers: Vec<Modifier>,
    pub damage_taken: u8,
}

impl CardRect {
    pub fn rotation(&self) -> f32 {
        if self.tapped {
            return std::f32::consts::FRAC_PI_2;
        }

        return 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct CellRect {
    pub id: u8,
    pub rect: Rect,
}
