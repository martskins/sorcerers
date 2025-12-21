use macroquad::{math::Rect, texture::Texture2D};
use sorcerers::card::{CardType, Modifier, Plane, Zone};

#[derive(Debug, Clone)]
pub struct CardDisplay {
    pub id: uuid::Uuid,
    pub zone: Zone,
    pub plane: Plane,
    pub card_type: CardType,
    pub tapped: bool,
    pub image: Texture2D,
    pub rect: Rect,
    pub rotation: f32,
    pub is_hovered: bool,
    pub is_selected: bool,
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone)]
pub struct CellDisplay {
    pub id: u8,
    pub rect: Rect,
}
