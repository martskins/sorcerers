use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width, screen_rect},
    input::Mouse,
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Ui, pos2, vec2};
use sorcerers::{
    card::CardData,
    networking::{self, message::ClientMessage},
};
use std::collections::HashSet;

const FONT_SIZE: f32 = 24.0;

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionOverlayBehaviour {
    Preview,
    Pick,
}

#[derive(Debug)]
pub struct SelectionOverlay {
    card_rects: Vec<CardRect>,
    prompt: String,
    behaviour: SelectionOverlayBehaviour,
    close: bool,
    player_id: uuid::Uuid,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    pickable_cards: HashSet<uuid::Uuid>,
}

impl SelectionOverlay {
    pub fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &uuid::Uuid,
        cards: Vec<&CardData>,
        pickable_cards: Vec<uuid::Uuid>,
        prompt: &str,
        behaviour: SelectionOverlayBehaviour,
    ) -> Self {
        let card_rects = match behaviour {
            SelectionOverlayBehaviour::Preview => Self::build_preview_rects(&cards),
            SelectionOverlayBehaviour::Pick => Self::build_pick_rects(&cards),
        };

        let pickable_cards = if pickable_cards.is_empty() {
            cards.iter().map(|card| card.id).collect()
        } else {
            pickable_cards.into_iter().collect()
        };

        Self {
            client,
            game_id: game_id.clone(),
            card_rects,
            prompt: prompt.to_string(),
            behaviour,
            player_id: player_id.clone(),
            close: false,
            pickable_cards,
        }
    }

    fn build_preview_rects(cards: &[&CardData]) -> Vec<CardRect> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        if cards.is_empty() {
            return Vec::new();
        }
        let card_spacing = 20.0;
        let card_count = cards.len();
        let cw = card_width().unwrap_or(80.0) * 2.0;
        let ch = card_height().unwrap_or(112.0) * 2.0;
        let cards_area_width = card_count as f32 * cw + (card_count as f32 - 1.0) * card_spacing;
        let cards_start_x = (sw - cards_area_width) / 2.0;
        let cards_y = (sh - ch) / 2.0 + 30.0;

        let mut rects = Vec::with_capacity(cards.len());
        for (idx, card) in cards.iter().enumerate() {
            let mut size = vec2(cw, ch);
            if card.is_site() {
                size = vec2(ch, cw);
            }
            let x = cards_start_x + idx as f32 * (size.x + card_spacing);
            rects.push(CardRect {
                image: None,
                rect: Rect::from_min_size(pos2(x, cards_y), size),
                is_hovered: false,
                is_selected: false,
                card: (*card).clone(),
            });
        }

        rects
    }

    fn build_pick_rects(cards: &[&CardData]) -> Vec<CardRect> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let card_count = cards.len();
        if card_count == 0 {
            return Vec::new();
        }

        let base_w = card_width().unwrap_or(80.0);
        let base_h = card_height().unwrap_or(112.0);
        let card_gap_x = 22.0;
        let row_step = (base_h * 0.18).max(18.0);
        let top_margin = 100.0;
        let bottom_margin = 80.0;
        let available_h = (sh - top_margin - bottom_margin).max(base_h);

        let cards_per_column = (((available_h - base_h) / row_step).floor() as usize + 1).max(1);
        let max_columns = (((sw - 80.0) + card_gap_x) / (base_w.max(base_h) + card_gap_x))
            .floor()
            .max(1.0) as usize;
        let mut columns = (card_count + cards_per_column - 1) / cards_per_column;
        columns = columns.clamp(1, max_columns.max(1));
        let cards_per_column = (card_count + columns - 1) / columns;

        let column_w = base_w.max(base_h);
        let total_width = columns as f32 * column_w + (columns as f32 - 1.0) * card_gap_x;
        let start_x = (sw - total_width) / 2.0;
        let start_y = top_margin;

        let mut rects = Vec::with_capacity(card_count);
        for (column_idx, column_cards) in cards.chunks(cards_per_column).enumerate() {
            let column_x = start_x + column_idx as f32 * (column_w + card_gap_x);
            for (row_idx, card) in column_cards.iter().enumerate() {
                let size = if card.is_site() {
                    vec2(base_h, base_w)
                } else {
                    vec2(base_w, base_h)
                };
                let x = column_x + (column_w - size.x) / 2.0;
                let y = start_y + row_idx as f32 * row_step;
                rects.push(CardRect {
                    image: None,
                    rect: Rect::from_min_size(pos2(x, y), size),
                    is_hovered: false,
                    is_selected: false,
                    card: (*card).clone(),
                });
            }
        }

        rects
    }

    fn hovered_card_index(&self, ctx: &Context) -> Option<usize> {
        let mouse_pos = Mouse::position(ctx)?;
        let mut hovered = None;
        for (idx, card_rect) in self.card_rects.iter().enumerate() {
            if card_rect.rect.contains(mouse_pos) {
                hovered = Some(idx);
            }
        }
        hovered
    }

    fn update_hover_state(&mut self, ctx: &Context) {
        let hovered = self.hovered_card_index(ctx);
        for card_rect in &mut self.card_rects {
            card_rect.is_hovered = false;
        }
        if let Some(idx) = hovered {
            if let Some(card_rect) = self.card_rects.get_mut(idx) {
                card_rect.is_hovered = true;
            }
        }
    }

    fn is_pickable(&self, card_id: &uuid::Uuid) -> bool {
        self.pickable_cards.contains(card_id)
    }

    fn hovered_card(&self) -> Option<&CardRect> {
        self.card_rects.iter().rev().find(|card| card.is_hovered)
    }

    fn render_hover_preview(&self, hovered: &CardRect, ui: &Ui, painter: &Painter) {
        let screen = screen_rect().unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0)));
        let mouse = Mouse::position(ui.ctx()).unwrap_or(hovered.rect.center());
        let preview_scale = 2.0;
        let preview_size = vec2(
            hovered.rect.width() * preview_scale,
            hovered.rect.height() * preview_scale,
        );
        let mut preview_pos = mouse + vec2(28.0, 28.0);
        if preview_pos.x + preview_size.x > screen.max.x - 8.0 {
            preview_pos.x = mouse.x - preview_size.x - 28.0;
        }
        if preview_pos.y + preview_size.y > screen.max.y - 8.0 {
            preview_pos.y = screen.max.y - preview_size.y - 8.0;
        }
        let min_x = screen.min.x + 8.0;
        let min_y = screen.min.y + 8.0;
        let max_x = (screen.max.x - preview_size.x - 8.0).max(min_x);
        let max_y = (screen.max.y - preview_size.y - 8.0).max(min_y);
        preview_pos.x = preview_pos.x.clamp(min_x, max_x);
        preview_pos.y = preview_pos.y.clamp(min_y, max_y);

        let preview_rect = Rect::from_min_size(preview_pos, preview_size);
        let preview_card = CardRect {
            rect: preview_rect,
            image: hovered.image.clone(),
            is_hovered: false,
            is_selected: false,
            card: hovered.card.clone(),
        };

        painter.rect_filled(
            preview_rect.expand(6.0),
            8.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 140),
        );
        render::draw_card(&preview_card, true, false, painter);
    }
}

impl Component for SelectionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        for card_rect in &mut self.card_rects {
            if card_rect.image.is_none() {
                card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
            }
        }
        self.update_hover_state(ctx);
        Ok(())
    }

    fn process_command(&mut self, _command: &ComponentCommand, _data: &mut GameData) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        !self.close
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::SelectionOverlay
    }

    fn process_input(
        &mut self,
        _in_turn: bool,
        data: &mut GameData,
        ctx: &Context,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if !Mouse::enabled() {
            return Ok(None);
        }

        self.update_hover_state(ctx);
        let clicked = Mouse::clicked(ctx);

        if !clicked {
            return Ok(None);
        }

        let hovered = match self.hovered_card() {
            Some(card) => card,
            None => return Ok(None),
        };

        match self.behaviour {
            SelectionOverlayBehaviour::Preview => {
                self.client.send(ClientMessage::ClickCard {
                    game_id: self.game_id,
                    player_id: self.player_id,
                    card_id: hovered.card.id,
                })?;
                self.close = true;
            }
            SelectionOverlayBehaviour::Pick => {
                if self.is_pickable(&hovered.card.id) {
                    self.client.send(ClientMessage::PickCard {
                        game_id: self.game_id,
                        player_id: self.player_id,
                        card_id: hovered.card.id,
                    })?;
                    self.close = true;
                }
            }
        }

        data.status = Status::Idle;

        Ok(None)
    }

    fn render(&mut self, _data: &mut GameData, ui: &mut Ui, painter: &Painter) -> anyhow::Result<()> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let title = ui.fonts(|f| {
            f.layout_no_wrap(
                self.prompt.clone(),
                egui::FontId::proportional(FONT_SIZE),
                Color32::WHITE,
            )
        });

        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );
        painter.galley(pos2(sw / 2.0 - title.size().x / 2.0, 30.0), title, Color32::WHITE);

        for card_rect in &self.card_rects {
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
            if self.behaviour == SelectionOverlayBehaviour::Pick && !self.is_pickable(&card_rect.card.id) {
                painter.rect_filled(card_rect.rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 110));
            }
        }

        if let Some(hovered) = self.hovered_card() {
            if self.behaviour == SelectionOverlayBehaviour::Pick {
                self.render_hover_preview(hovered, ui, painter);
            }
        }

        if self.behaviour == SelectionOverlayBehaviour::Preview {
            let close_button_pos = pos2(sw / 2.0 - 50.0, sh - 70.0);
            egui::Area::new(egui::Id::new("selection_close_btn"))
                .fixed_pos(close_button_pos)
                .show(ui.ctx(), |ui| {
                    let close = egui::Button::new(egui::RichText::new("Close").size(22.0).color(Color32::WHITE))
                        .min_size(vec2(120.0, 44.0));
                    if ui.add(close).clicked() {
                        self.close = true;
                    }
                });
        }

        Ok(())
    }
}
