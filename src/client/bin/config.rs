use std::sync::{OnceLock, RwLock};

use macroquad::{
    math::{Rect, Vec2},
    window::{screen_height, screen_width},
};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();
pub const CARD_IN_PLAY_SCALE: f32 = 0.6;
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

pub fn card_width() -> f32 {
    realm_rect().w / 10.0
}

pub fn card_height() -> f32 {
    let height = (screen_rect().w - 200.0) / 10.0 / CARD_ASPECT_RATIO;
    if height > 200.0 { 200.0 } else { height }
}

pub fn spell_dimensions() -> Vec2 {
    Vec2::new(card_width(), card_height())
}

pub fn site_dimensions() -> Vec2 {
    Vec2::new(card_height(), card_width())
}

fn hand_space_height() -> f32 {
    card_height() + 20.0
}

pub fn screen_rect() -> Rect {
    SCREEN_RECT
        .get_or_init(|| RwLock::new(Rect::new(0.0, 0.0, screen_width(), screen_height())))
        .read()
        .unwrap()
        .clone()
}

pub fn hand_rect() -> Rect {
    Rect::new(realm_rect().x, realm_rect().h, realm_rect().w, hand_space_height())
}

pub fn realm_rect() -> Rect {
    let screen_rect = screen_rect();
    Rect::new(200.0, 0.0, screen_rect.w - 200.0, screen_rect.h - hand_space_height()) // Last parame is the hand's area height
}

pub fn cell_rect(id: u8, mirror: bool) -> Rect {
    let realm_rect = realm_rect();
    // id 1 is bottom left, id 5 is bottom right, id 16 is top right
    let idx = id - 1;
    let mut col = idx % 5;
    let mut row = 3 - (idx / 5); // invert row for bottom-up indexing

    if mirror {
        col = 4 - col; // mirror horizontally
    }
    if mirror {
        row = 3 - row; // mirror vertically
    }

    Rect::new(
        realm_rect.x + col as f32 * (realm_rect.w / 5.0),
        realm_rect.y + row as f32 * (realm_rect.h / 4.0),
        realm_rect.w / 5.0,
        realm_rect.h / 4.0,
    )
}
