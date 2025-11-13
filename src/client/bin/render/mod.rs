use macroquad::math::Rect;
use sorcerers::card::Card;

#[derive(Debug, Clone)]
pub struct CardDisplay {
    pub card: Card,
    pub rect: Rect,
    pub is_hovered: bool,
    pub is_selected: bool,
}

#[derive(Debug, Clone)]
pub struct CellDisplay {
    pub id: u8,
    pub rect: Rect,
}
