use std::sync::{OnceLock, RwLock};

use macroquad::{
    math::Rect,
    window::{screen_height, screen_width},
};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

pub fn card_width() -> f32 {
    realm_rect().w / 10.0
}

pub fn card_height() -> f32 {
    let height = (screen_rect().w - 200.0) / 10.0 / CARD_ASPECT_RATIO;
    if height > 200.0 { 200.0 } else { height }
}

fn hand_space_height() -> f32 {
    card_height() + 20.0
}

pub fn event_log_rect() -> Rect {
    Rect::new(0.0, 0.0, screen_width() * 0.8, 100.0)
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
    let min_x = 200.0;
    let max_x = 250.0;
    let mut x = screen_rect.w * 0.2;
    if x < min_x {
        x = min_x;
    } else if x > max_x {
        x = max_x;
    }
    Rect::new(x, 0.0, screen_rect.w - x, screen_rect.h - hand_space_height())
}
