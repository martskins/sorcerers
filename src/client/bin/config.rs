use egui::{Rect, Vec2, pos2};
use std::sync::{OnceLock, RwLock};

pub static SCREEN_RECT: OnceLock<RwLock<Rect>> = OnceLock::new();
pub const CARD_ASPECT_RATIO: f32 = 384.0 / 537.0;

pub fn screen_rect() -> anyhow::Result<Rect> {
    Ok(*SCREEN_RECT
        .get_or_init(|| {
            RwLock::new(Rect::from_min_size(
                pos2(0.0, 0.0),
                Vec2::new(1280.0, 720.0),
            ))
        })
        .read()
        .map_err(|e| anyhow::anyhow!("lock: {}", e))?)
}

pub fn card_width() -> anyhow::Result<f32> {
    Ok(screen_rect()?.width() / 11.5)
}

pub fn card_height() -> anyhow::Result<f32> {
    let height = card_width()? / CARD_ASPECT_RATIO;
    Ok(height.clamp(0.0, 200.0))
}

fn hand_space_height() -> anyhow::Result<f32> {
    Ok(card_height()? + 20.0)
}

pub fn event_log_rect() -> Rect {
    let sr = screen_rect().unwrap_or(Rect::from_min_size(
        pos2(0.0, 0.0),
        Vec2::new(1280.0, 720.0),
    ));
    let w = (sr.width() * 0.5).clamp(300.0, 500.0);
    Rect::from_center_size(sr.center(), Vec2::new(w, 250.0))
}

pub fn hand_rect() -> anyhow::Result<Rect> {
    let sr = screen_rect()?;
    let hand_height = hand_space_height()?;
    Ok(Rect::from_min_size(
        pos2(0.0, sr.max.y - hand_height),
        Vec2::new(sr.width(), hand_height),
    ))
}

pub fn realm_rect() -> anyhow::Result<Rect> {
    let sr = screen_rect()?;
    // The realm geometry carries its own inset. Reserve a slim left gutter so
    // its outer rail never crowds the persistent player-status rail.
    Ok(Rect::from_min_max(
        pos2(sr.min.x + 36.0, sr.min.y),
        sr.max,
    ))
}
