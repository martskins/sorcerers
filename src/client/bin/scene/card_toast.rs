use crate::{
    config::{CARD_ASPECT_RATIO, realm_rect, screen_rect},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, FontId, Painter, Rect, TextureHandle, pos2, vec2};
use sorcerers::card::CardData;

const TOAST_DURATION: f64 = 4.5;
/// How many seconds before the end the fade-out starts.
const TOAST_FADE_SECS: f64 = 1.2;
/// Slide-in duration in seconds.
const TOAST_SLIDE_SECS: f64 = 0.22;

const TOAST_CARD_W: f32 = 108.0;
const TOAST_CARD_H: f32 = TOAST_CARD_W / CARD_ASPECT_RATIO;
const TOAST_PAD: f32 = 8.0;
const TOAST_LABEL_H: f32 = 22.0;
const TOAST_MARGIN: f32 = 14.0;

/// Brief non-blocking card-played notification that slides in from the right,
/// lingers for a few seconds, then fades out.
pub struct CardToast {
    pub card: CardData,
    pub description: String,
    /// Initialized lazily to `ctx.input(|i| i.time)` on first render so we
    /// don't need to thread `ctx` through `process_message`.
    start_time: Option<f64>,
    image: Option<TextureHandle>,
}

impl CardToast {
    pub fn new(card: CardData, description: String) -> Self {
        Self {
            card,
            description,
            start_time: None,
            image: None,
        }
    }

    /// Returns the alpha (0–255) for the current frame, applying the fade-out.
    fn alpha(&self, elapsed: f64) -> u8 {
        let remaining = TOAST_DURATION - elapsed;
        if remaining <= 0.0 {
            return 0;
        }
        if remaining < TOAST_FADE_SECS {
            ((remaining / TOAST_FADE_SECS) * 255.0).clamp(0.0, 255.0) as u8
        } else {
            255
        }
    }

    /// Returns the horizontal slide offset (positive = off screen to the right).
    fn slide_offset(&self, elapsed: f64) -> f32 {
        if elapsed >= TOAST_SLIDE_SECS {
            return 0.0;
        }
        let t = (elapsed / TOAST_SLIDE_SECS) as f32;
        let ease = 1.0 - (1.0 - t).powi(3);
        (TOAST_CARD_W + TOAST_PAD * 2.0 + TOAST_MARGIN) * (1.0 - ease)
    }

    /// Render the toast. Returns `true` while still active.
    pub fn render(&mut self, ctx: &Context, painter: &Painter) -> bool {
        let now = ctx.input(|i| i.time);
        let start = *self.start_time.get_or_insert(now);
        let elapsed = now - start;

        if elapsed >= TOAST_DURATION {
            return false;
        }

        // Request continuous repaints while active so the animation & fade run.
        ctx.request_repaint_after(std::time::Duration::from_millis(30));

        // Load texture lazily.
        if self.image.is_none() {
            self.image = TextureCache::get_card_texture_blocking(&self.card, ctx);
        }

        let alpha = self.alpha(elapsed);
        let slide = self.slide_offset(elapsed);

        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let realm_r = realm_rect().unwrap_or(Rect::ZERO);

        // ── Toast rect ────────────────────────────────────────────
        let mut total_w = TOAST_CARD_W + TOAST_PAD * 2.0;
        let mut total_h = TOAST_CARD_H + TOAST_PAD * 2.0;
        if self.card.is_site() {
            std::mem::swap(&mut total_w, &mut total_h);
        }

        let base_x = sr.width() - total_w - TOAST_MARGIN + slide;
        let base_y = realm_r.max.y - total_h - TOAST_MARGIN;
        let toast_rect = Rect::from_min_size(pos2(base_x, base_y), vec2(total_w, total_h));

        // ── Background panel ──────────────────────────────────────
        painter.rect_filled(
            toast_rect,
            8.0,
            Color32::from_rgba_unmultiplied(8, 10, 22, (alpha as f32 * 0.88) as u8),
        );
        painter.rect_stroke(
            toast_rect,
            8.0,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(60, 80, 140, alpha)),
            egui::StrokeKind::Outside,
        );

        // ── Card image ────────────────────────────────────────────

        let mut card_size = vec2(TOAST_CARD_W, TOAST_CARD_H);
        if self.card.is_site() {
            std::mem::swap(&mut card_size.x, &mut card_size.y);
        }
        let card_rect =
            Rect::from_min_size(pos2(base_x + TOAST_PAD, base_y + TOAST_PAD), card_size);
        match &self.image {
            Some(tex) => {
                painter.image(
                    tex.id(),
                    card_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );
            }
            None => {
                painter.rect_filled(
                    card_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(40, 44, 64, alpha),
                );
            }
        }

        // ── Description label ─────────────────────────────────────
        let label_y = base_y + TOAST_PAD + TOAST_CARD_H + 4.0;
        painter.text(
            pos2(base_x + total_w / 2.0, label_y),
            egui::Align2::CENTER_TOP,
            &self.description,
            FontId::proportional(10.5),
            Color32::from_rgba_unmultiplied(180, 195, 230, alpha),
        );

        // ── Hover → show large preview on the left sidebar ───────
        let hovered = ctx
            .input(|i| i.pointer.hover_pos())
            .map_or(false, |p| toast_rect.contains(p));

        if hovered {
            if let Some(ref tex) = self.image {
                let sidebar_w = realm_r.min.x;
                let preview_w = sidebar_w - 8.0;
                let preview_h = preview_w / CARD_ASPECT_RATIO;
                let preview_y = sr.height() / 2.0 - preview_h / 2.0;
                let preview_rect =
                    Rect::from_min_size(pos2(4.0, preview_y), vec2(preview_w, preview_h));
                painter.image(
                    tex.id(),
                    preview_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }

        true
    }
}
