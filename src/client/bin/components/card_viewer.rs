use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    render::CardRect,
    scene::game::GameData,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};

const THUMB_W: f32 = 80.0;
const THUMB_H: f32 = THUMB_W / CARD_ASPECT_RATIO;
const CARD_PAD: f32 = 10.0;
/// Height of the visible strip exposed by each backing card in a stack.
const STACK_STRIP: f32 = 22.0;
/// Maximum cards per column before a new column is started.
const MAX_PER_COLUMN: usize = 10;

#[derive(Debug)]
pub struct CardViewerComponent {
    visible: bool,
    title: String,
    cards: Vec<CardRect>,
}

impl CardViewerComponent {
    pub fn new() -> Self {
        Self {
            visible: false,
            title: "Cards".to_string(),
            cards: Vec::new(),
        }
    }
}

impl Component for CardViewerComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }
        for card_rect in &mut self.cards {
            if card_rect.image.is_none() {
                card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
            }
        }
        TextureCache::flush_blocking(ctx);
        let _ = data;
        Ok(())
    }

    fn render(
        &mut self,
        _data: &mut GameData,
        ui: &mut Ui,
        _painter: &Painter,
    ) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }

        let mut open = self.visible;
        let mut hovered_idx: Option<usize> = None;

        egui::Window::new(self.title.clone())
            .open(&mut open)
            .movable(true)
            .resizable(true)
            .min_width(THUMB_W + CARD_PAD * 2.0)
            .min_height(THUMB_H + CARD_PAD * 2.0)
            .default_size(vec2(600.0, 400.0))
            .show(ui.ctx(), |ui| {
                if self.cards.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("(empty)")
                                .color(Color32::GRAY)
                                .size(14.0),
                        );
                    });
                    return;
                }

                // Reset hover state before detection.
                for card in &mut self.cards {
                    card.is_hovered = false;
                }

                let total = self.cards.len();
                let num_cols = total.div_ceil(MAX_PER_COLUMN);
                let mouse_pos = ui.ctx().input(|i| i.pointer.hover_pos());

                // `auto_shrink([false, false])` lets the user drag the window
                // larger than its content without it snapping back.
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing = vec2(CARD_PAD, 0.0);

                            for col_idx in 0..num_cols {
                                let start = col_idx * MAX_PER_COLUMN;
                                let end = (start + MAX_PER_COLUMN).min(total);
                                let n = end - start;

                                let col_h = STACK_STRIP * n.saturating_sub(1) as f32 + THUMB_H;
                                let (col_rect, _) =
                                    ui.allocate_exact_size(vec2(THUMB_W, col_h), Sense::hover());

                                // Hover detection (done before drawing so the highlighted
                                // border is correct in the same frame).
                                if let Some(mp) = mouse_pos {
                                    for local_i in (0..n).rev() {
                                        let gi = start + local_i;
                                        let y =
                                            col_rect.min.y + local_i as f32 * STACK_STRIP;
                                        let visible_h = if local_i == n - 1 {
                                            THUMB_H
                                        } else {
                                            STACK_STRIP
                                        };
                                        let hit = Rect::from_min_size(
                                            pos2(col_rect.min.x, y),
                                            vec2(THUMB_W, visible_h),
                                        );
                                        if hit.contains(mp) {
                                            self.cards[gi].is_hovered = true;
                                            hovered_idx = Some(gi);
                                            ui.ctx().set_cursor_icon(
                                                egui::CursorIcon::PointingHand,
                                            );
                                            break;
                                        }
                                    }
                                }

                                if !ui.is_rect_visible(col_rect) {
                                    continue;
                                }

                                let painter = ui.painter();

                                // Pass 1: draw all card images / placeholders back-to-front.
                                // Each successive card covers the lower portion of the one
                                // before it, leaving only the STACK_STRIP strip visible.
                                for local_i in 0..n {
                                    let gi = start + local_i;
                                    let y = col_rect.min.y + local_i as f32 * STACK_STRIP;
                                    let card_rect = Rect::from_min_size(
                                        pos2(col_rect.min.x, y),
                                        vec2(THUMB_W, THUMB_H),
                                    );
                                    self.cards[gi].rect = card_rect;

                                    if let Some(ref tex) = self.cards[gi].image {
                                        painter.image(
                                            tex.id(),
                                            card_rect,
                                            Rect::from_min_max(
                                                pos2(0.0, 0.0),
                                                pos2(1.0, 1.0),
                                            ),
                                            Color32::WHITE,
                                        );
                                    } else {
                                        painter.rect_filled(
                                            card_rect,
                                            4.0,
                                            Color32::DARK_GRAY,
                                        );
                                        // For the front card with no image, show name in centre.
                                        if local_i == n - 1 {
                                            painter.text(
                                                card_rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                self.cards[gi].card.get_name(),
                                                egui::FontId::proportional(9.0),
                                                Color32::LIGHT_GRAY,
                                            );
                                        }
                                    }
                                }

                                // Pass 2: draw name strip overlays for backing cards and
                                // borders for all cards.
                                for local_i in 0..n {
                                    let gi = start + local_i;
                                    let y = col_rect.min.y + local_i as f32 * STACK_STRIP;
                                    let card_rect = self.cards[gi].rect;

                                    if local_i < n - 1 {
                                        // Semi-transparent overlay over the visible strip.
                                        let strip = Rect::from_min_size(
                                            pos2(col_rect.min.x, y),
                                            vec2(THUMB_W, STACK_STRIP),
                                        );
                                        painter.rect_filled(
                                            strip,
                                            4.0,
                                            Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                                        );
                                        painter.text(
                                            strip.center(),
                                            egui::Align2::CENTER_CENTER,
                                            self.cards[gi].card.get_name(),
                                            egui::FontId::proportional(9.0),
                                            Color32::LIGHT_GRAY,
                                        );
                                    }

                                    let border_color = if self.cards[gi].is_hovered {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(100)
                                    };
                                    painter.rect_stroke(
                                        card_rect,
                                        4.0,
                                        Stroke::new(1.0, border_color),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                        });
                    });
            });

        // Hover preview rendered at the Tooltip layer so it floats above the window.
        if let Some(idx) = hovered_idx {
            let tex = &self.cards[idx].image;
            crate::render::draw_card_preview(ui, tex.as_ref()).unwrap();
        }

        self.visible = open;
        Ok(())
    }

    fn process_input(
        &mut self,
        _in_turn: bool,
        _data: &mut GameData,
        _ctx: &Context,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        Ok(None)
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        if let ComponentCommand::OpenCardViewer { title, cards } = command {
            self.title = title.clone();
            self.cards = cards
                .iter()
                .filter_map(|id| data.cards.iter().find(|c| c.id == *id))
                .map(|card| CardRect {
                    rect: Rect::ZERO,
                    image: None,
                    is_hovered: false,
                    is_selected: false,
                    card: card.clone(),
                })
                .collect();
            self.visible = true;
        }
        Ok(())
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::CardViewer
    }
}
