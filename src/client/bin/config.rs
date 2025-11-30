use std::sync::{OnceLock, RwLock};

use macroquad::{
    math::{Rect, Vec2},
    window::{screen_height, screen_width},
};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();

pub const CARD_OFFSET_X: f32 = 50.0;
pub const CARD_IN_PLAY_SCALE: f32 = 0.6;
pub const SPELLBOOK_IMAGE: &str = "assets/images/cards/Spell Back.webp";
pub const ATLASBOOK_IMAGE: &str = "assets/images/cards/Site Back.webp";
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

pub fn card_width() -> f32 {
    realm_rect().w / 10.0
}

pub fn card_height() -> f32 {
    realm_rect().w / 10.0 / CARD_ASPECT_RATIO
}

pub fn spell_dimensions() -> Vec2 {
    Vec2::new(card_width(), card_height())
}

pub fn site_dimensions() -> Vec2 {
    Vec2::new(card_height(), card_width())
}

pub fn screen_rect() -> Rect {
    SCREEN_RECT
        .get_or_init(|| RwLock::new(Rect::new(0.0, 0.0, screen_width(), screen_height())))
        .read()
        .unwrap()
        .clone()
}

pub fn hand_rect() -> Rect {
    let screen_rect = screen_rect();
    Rect::new((screen_rect.w / 2.0) - 150.0, screen_rect.h - 200.0, 300.0, 200.0)
}

pub fn realm_rect() -> Rect {
    let screen_rect = screen_rect();
    Rect::new(100.0, 0.0, screen_rect.w - 200.0, screen_rect.h - hand_rect().h)
}

pub fn spellbook_rect() -> Rect {
    let realm_rect = realm_rect();
    Rect::new(
        realm_rect.x + realm_rect.w + 10.0,
        realm_rect.y + realm_rect.h - 2.0 * card_height() - 30.0,
        card_width(),
        card_height(),
    )
}

pub fn cemetery_rect() -> Rect {
    let realm_rect = realm_rect();
    Rect::new(
        realm_rect.x + realm_rect.w + 10.0,
        realm_rect.y + realm_rect.h - card_height() - 10.0,
        card_width(),
        card_height(),
    )
}

pub fn atlasbook_rect() -> Rect {
    let realm_rect = realm_rect();
    Rect::new(
        realm_rect.x + realm_rect.w + 10.0,
        realm_rect.y + realm_rect.h - 3.0 * card_height() - 10.0,
        card_height(),
        card_width(),
    )
}

pub fn cell_rect(id: u8, mirror: bool) -> Rect {
    let realm_rect = realm_rect();
    let mut col = id % 5;
    if mirror {
        col = 4 - col;
    }
    let mut row = id / 5;
    if mirror {
        row = 3 - row;
    }
    Rect::new(
        realm_rect.x + col as f32 * (realm_rect.w / 5.0),
        realm_rect.y + row as f32 * (realm_rect.h / 4.0),
        realm_rect.w / 5.0,
        realm_rect.h / 4.0,
    )
}
