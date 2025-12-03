use macroquad::{math::Rect, texture::Texture2D};
use sorcerers::{card::Zone, game::PlayerId};

#[derive(Debug, Clone)]
pub struct CardDisplay {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: PlayerId,
    pub zone: Zone,
    pub tapped: bool,
    pub image: Texture2D,
    pub rect: Rect,
    pub rotation: f32,
    pub is_hovered: bool,
    pub is_selected: bool,
}

#[derive(Debug, Clone)]
pub struct CellDisplay {
    pub id: u8,
    pub rect: Rect,
}
