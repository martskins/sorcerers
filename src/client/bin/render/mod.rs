use macroquad::{math::Rect, texture::Texture2D};
use sorcerers::card::{Modifier, Zone};

#[derive(Debug, Clone)]
pub struct CardRect {
    pub id: uuid::Uuid,
    pub zone: Zone,
    pub tapped: bool,
    pub image: Texture2D,
    pub rect: Rect,
    pub rotation: f32,
    pub is_hovered: bool,
    pub is_selected: bool,
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone)]
pub struct CellRect {
    pub id: u8,
    pub rect: Rect,
}
