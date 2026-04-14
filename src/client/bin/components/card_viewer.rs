use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    render::CardRect,
    scene::game::GameData,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};
use sorcerers::game::PlayerId;

const THUMB_W: f32 = 80.0;
const THUMB_H: f32 = THUMB_W / CARD_ASPECT_RATIO;
const CARD_PAD: f32 = 6.0;
const PREVIEW_SCALE: f32 = 3.0;

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
        // Keep flushing so textures arrive promptly.
        TextureCache::flush_blocking(ctx);
        let _ = data;
        Ok(())
    }

    fn render(
        &mut self,
        _data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
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
            .min_width(600.0)
            .min_height(400.0)
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

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let card_size = vec2(THUMB_W, THUMB_H);

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(CARD_PAD, CARD_PAD);
                        for (i, card_rect) in self.cards.iter_mut().enumerate() {
                            let (rect, response) =
                                ui.allocate_exact_size(card_size, Sense::hover());
                            card_rect.rect = rect;
                            card_rect.is_hovered = response.hovered();
                            if response.hovered() {
                                hovered_idx = Some(i);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if ui.is_rect_visible(rect) {
                                let painter = ui.painter_at(rect);
                                if let Some(ref tex) = card_rect.image {
                                    painter.image(
                                        tex.id(),
                                        rect,
                                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    painter.rect_filled(rect, 4.0, Color32::DARK_GRAY);
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        card_rect.card.get_name(),
                                        egui::FontId::proportional(9.0),
                                        Color32::LIGHT_GRAY,
                                    );
                                }
                                let border_color = Color32::from_gray(100);
                                painter.rect_stroke(
                                    rect,
                                    4.0,
                                    Stroke::new(1.0, border_color),
                                    egui::StrokeKind::Outside,
                                );
                            }
                        }
                    });
                });
            });

        // Hover preview: show a larger floating card above everything (Tooltip layer).
        if let Some(idx) = hovered_idx {
            let tex = &self.cards[idx].image;
            let pointer_pos = ui.input(|i| i.pointer.latest_pos()).unwrap_or_default();
            let preview_painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("card_viewer_hover_preview"),
            ));
            crate::render::draw_card_preview(tex.as_ref(), pointer_pos, &preview_painter).unwrap();
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
