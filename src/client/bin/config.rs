use egui::{Rect, Vec2, pos2};
use std::sync::{OnceLock, RwLock};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

/// Pixels reserved at the bottom of the left sidebar for the player status
/// panel (panel height 96 + 4 px bottom margin + 4 px gap = 104).
pub const SIDEBAR_PANEL_RESERVED: f32 = 104.0;

pub fn screen_rect() -> anyhow::Result<Rect> {
    Ok(*SCREEN_RECT
        .get_or_init(|| RwLock::new(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1280.0, 720.0))))
        .read()
        .map_err(|e| anyhow::anyhow!("lock: {}", e))?)
}

pub fn card_width() -> anyhow::Result<f32> {
    Ok(realm_rect()?.width() / 10.0)
}

pub fn card_height() -> anyhow::Result<f32> {
    let height = (screen_rect()?.width() - 200.0) / 10.0 / CARD_ASPECT_RATIO;
    Ok(height.clamp(0.0, 200.0))
}

fn hand_space_height() -> anyhow::Result<f32> {
    Ok(card_height()? + 20.0)
}

pub fn event_log_rect() -> Rect {
    let sr = screen_rect().unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1280.0, 720.0)));
    let w = (sr.width() * 0.5).clamp(300.0, 500.0);
    Rect::from_center_size(sr.center(), Vec2::new(w, 250.0))
}

pub fn hand_rect() -> anyhow::Result<Rect> {
    let rr = realm_rect()?;
    Ok(Rect::from_min_size(pos2(rr.min.x, rr.max.y), Vec2::new(rr.width(), hand_space_height()?)))
}

pub fn realm_rect() -> anyhow::Result<Rect> {
    let sr = screen_rect()?;
    let x = (sr.width() * 0.2).clamp(200.0, 250.0);
    let hsh = hand_space_height()?;
    Ok(Rect::from_min_max(
        pos2(x, 0.0),
        pos2(sr.width(), sr.height() - hsh),
    ))
}
