use std::sync::{OnceLock, RwLock};

use macroquad::{
    math::Rect,
    window::{screen_height, screen_width},
};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

pub fn card_width() -> anyhow::Result<f32> {
    Ok(realm_rect()?.w / 10.0)
}

pub fn card_height() -> anyhow::Result<f32> {
    let height = (screen_rect()?.w - 200.0) / 10.0 / CARD_ASPECT_RATIO;
    Ok(height.clamp(0.0, 200.0))
}

fn hand_space_height() -> anyhow::Result<f32> {
    Ok(card_height()? + 20.0)
}

pub fn event_log_rect() -> Rect {
    Rect::new(0.0, 0.0, screen_width() * 0.8, 200.0)
}

pub fn screen_rect() -> anyhow::Result<Rect> {
    Ok(SCREEN_RECT
        .get_or_init(|| RwLock::new(Rect::new(0.0, 0.0, screen_width(), screen_height())))
        .read()
        .map_err(|e| anyhow::anyhow!("failed to lock for read: {}", e))?
        .clone())
}

pub fn hand_rect() -> anyhow::Result<Rect> {
    Ok(Rect::new(
        realm_rect()?.x,
        realm_rect()?.h,
        realm_rect()?.w,
        hand_space_height()?,
    ))
}

pub fn realm_rect() -> anyhow::Result<Rect> {
    let screen_rect = screen_rect()?;
    let min_x = 200.0;
    let max_x = 250.0;
    let x = (screen_rect.w * 0.2).clamp(min_x, max_x);
    Ok(Rect::new(
        x,
        0.0,
        screen_rect.w - x,
        screen_rect.h - hand_space_height()?,
    ))
}
