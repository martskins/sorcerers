use crate::{
    config::{CARD_ASPECT_RATIO, screen_rect},
    render,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, FontId, Painter, Rect, TextureHandle, pos2, vec2};
use sorcerers::card::CardData;

/// Duration of a card-played toast (longer so the image can be appreciated).
const CARD_TOAST_DURATION: f64 = 4.5;
/// Duration of a plain text event toast.
const EVENT_TOAST_DURATION: f64 = 3.0;
/// How many seconds before the end the fade-out starts.
const TOAST_FADE_SECS: f64 = 1.2;
/// Slide-in duration in seconds.
const TOAST_SLIDE_SECS: f64 = 0.22;

const TOAST_CARD_W: f32 = 108.0;
const TOAST_CARD_H: f32 = TOAST_CARD_W / CARD_ASPECT_RATIO;
const TOAST_PAD: f32 = 8.0;
pub const TOAST_MARGIN: f32 = 14.0;
const TOAST_FONT_SIZE: f32 = 10.5;

enum ToastKind {
    /// A card was played — show the card image plus a description.
    Card {
        card: CardData,
        image: Option<TextureHandle>,
    },
    /// A generic game event — show a text line only.
    Event,
}

/// Brief non-blocking notification that slides in from the right,
/// lingers for a few seconds, then fades out.
///
/// Two variants exist:
/// - **Card** — shows the card image and a description (for `CardPlayed` messages).
/// - **Event** — shows only a text line (for every `LogEvent` message).
pub struct CardToast {
    pub description: String,
    /// Initialized lazily to `ctx.input(|i| i.time)` on first render so we
    /// don't need to thread `ctx` through `process_message`.
    start_time: Option<f64>,
    duration: f64,
    kind: ToastKind,
}

impl CardToast {
    /// Create a toast that shows a card image and description.
    pub fn new_card(card: CardData, description: String) -> Self {
        Self {
            description,
            start_time: None,
            duration: CARD_TOAST_DURATION,
            kind: ToastKind::Card { card, image: None },
        }
    }

    /// Create a text-only toast for a game event.
    pub fn new_event(description: String) -> Self {
        Self {
            description,
            start_time: None,
            duration: EVENT_TOAST_DURATION,
            kind: ToastKind::Event,
        }
    }

    /// The rendered height of this toast (used for vertical stacking).
    /// Measures the wrapped text so the background always fits.
    pub fn height(&self, ctx: &Context) -> f32 {
        let inner_w = toast_inner_width(&self.kind);
        let text_h = measure_text_height(ctx, &self.description, inner_w);
        match &self.kind {
            ToastKind::Card { card, .. } => {
                let (_, card_h) = card_dimensions(card);
                TOAST_PAD + card_h + TOAST_PAD + text_h + TOAST_PAD
            }
            ToastKind::Event => TOAST_PAD + text_h + TOAST_PAD,
        }
    }

    /// Returns the alpha (0–255) for the current frame, applying the fade-out.
    fn alpha(&self, elapsed: f64) -> u8 {
        let remaining = self.duration - elapsed;
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
        (toast_panel_width(&self.kind) + TOAST_MARGIN) * (1.0 - ease)
    }

    /// Render the toast at the given `base_y` (top-left y of the toast rect).
    /// Returns `true` while still active.
    pub fn render(&mut self, ctx: &Context, ui: &egui::Ui, base_y: f32) -> bool {
        let now = ctx.input(|i| i.time);
        let start = *self.start_time.get_or_insert(now);
        let elapsed = now - start;

        if elapsed >= self.duration {
            return false;
        }

        // Request continuous repaints while active so the animation & fade run.
        ctx.request_repaint_after(std::time::Duration::from_millis(30));

        let alpha = self.alpha(elapsed);
        let slide = self.slide_offset(elapsed);
        let text_color = Color32::from_rgba_unmultiplied(180, 195, 230, alpha);

        let sr = screen_rect().unwrap_or(Rect::ZERO);
        let total_w = toast_panel_width(&self.kind);
        let inner_w = toast_inner_width(&self.kind);
        let total_h = self.height(ctx);

        let base_x = sr.width() - total_w - TOAST_MARGIN + slide;
        let toast_rect = Rect::from_min_size(pos2(base_x, base_y), vec2(total_w, total_h));

        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("card_viewer_hover_preview"),
        ));

        // ── Background panel ──────────────────────────────────────
        painter.rect_filled(
            toast_rect,
            6.0,
            Color32::from_rgba_unmultiplied(8, 10, 22, (alpha as f32 * 0.88) as u8),
        );
        painter.rect_stroke(
            toast_rect,
            6.0,
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(60, 80, 140, alpha)),
            egui::StrokeKind::Outside,
        );

        match &mut self.kind {
            ToastKind::Card { card, image } => {
                // Load texture lazily.
                if image.is_none() {
                    *image = TextureCache::get_card_texture_blocking(card, ctx);
                }

                let (card_w, card_h) = card_dimensions(card);
                let card_rect = Rect::from_min_size(
                    pos2(base_x + TOAST_PAD, base_y + TOAST_PAD),
                    vec2(card_w, card_h),
                );

                match image {
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

                // Description label below the card image, sized to fit the panel.
                let desc_galley = ctx.fonts(|f| {
                    f.layout(
                        self.description.clone(),
                        FontId::proportional(TOAST_FONT_SIZE),
                        text_color,
                        inner_w,
                    )
                });
                painter.galley(
                    pos2(base_x + TOAST_PAD, base_y + TOAST_PAD + card_h + TOAST_PAD),
                    desc_galley,
                    text_color,
                );

                // Hover → show large preview on the left sidebar.
                let hovered = ctx
                    .input(|i| i.pointer.hover_pos())
                    .map_or(false, |p| toast_rect.contains(p));

                if hovered {
                    render::draw_sidebar_card_preview(ui, image.as_ref()).ok();
                }
            }
            ToastKind::Event => {
                // Text laid out to wrap within the inner width; panel height matches.
                let galley = ctx.fonts(|f| {
                    f.layout(
                        self.description.clone(),
                        FontId::proportional(TOAST_FONT_SIZE),
                        text_color,
                        inner_w,
                    )
                });
                // Vertically centre the text block inside the panel.
                let text_y = base_y + (total_h - galley.size().y) / 2.0;
                painter.galley(pos2(base_x + TOAST_PAD, text_y), galley, text_color);
            }
        }

        true
    }
}

/// Returns (width, height) of the card image region inside the toast.
fn card_dimensions(card: &CardData) -> (f32, f32) {
    if card.is_site() {
        (TOAST_CARD_H, TOAST_CARD_W)
    } else {
        (TOAST_CARD_W, TOAST_CARD_H)
    }
}

/// Total outer width of the toast panel.
fn toast_panel_width(kind: &ToastKind) -> f32 {
    toast_inner_width(kind) + TOAST_PAD * 2.0
}

/// Inner width available for text / card image inside the panel.
fn toast_inner_width(kind: &ToastKind) -> f32 {
    match kind {
        ToastKind::Card { card, .. } => {
            let (w, _) = card_dimensions(card);
            w
        }
        ToastKind::Event => TOAST_CARD_W,
    }
}

/// Measures the height of `text` when wrapped at `wrap_width` pixels.
fn measure_text_height(ctx: &Context, text: &str, wrap_width: f32) -> f32 {
    ctx.fonts(|f| {
        f.layout(
            text.to_string(),
            FontId::proportional(TOAST_FONT_SIZE),
            Color32::WHITE,
            wrap_width,
        )
        .size()
        .y
    })
}
