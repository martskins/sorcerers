use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width, screen_rect},
    input::Mouse,
    render::{self, CardRect},
    scene::game::GameData,
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Ui, pos2, vec2};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{self, message::ClientMessage},
};

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
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
}

impl SelectionOverlay {
    pub fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        cards: Vec<&CardData>,
        prompt: &str,
        behaviour: SelectionOverlayBehaviour,
    ) -> Self {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
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
                image: None, // loaded lazily
                rect: Rect::from_min_size(pos2(x, cards_y), size),
                is_hovered: false,
                is_selected: false,
                card: (*card).clone(),
            });
        }

        Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt: prompt.to_string(),
            behaviour,
            player_id: player_id.clone(),
            close: false,
        }
    }
}

impl Component for SelectionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        let mouse_pos = Mouse::position(ctx);
        for cell_rect in &mut self.card_rects {
            cell_rect.is_hovered = mouse_pos.map_or(false, |p| cell_rect.rect.contains(p));
            if cell_rect.image.is_none() {
                cell_rect.image = TextureCache::get_card_texture_blocking(&cell_rect.card, ctx);
            }
        }
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
        _data: &mut GameData,
        ctx: &Context,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if !Mouse::enabled() {
            return Ok(None);
        }

        let mp = Mouse::position(ctx).unwrap_or_default();
        let clicked = Mouse::clicked(ctx);

        for rect in &self.card_rects {
            if rect.rect.contains(mp) && clicked {
                match self.behaviour {
                    SelectionOverlayBehaviour::Preview => {
                        self.client.send(ClientMessage::ClickCard {
                            game_id: self.game_id,
                            player_id: self.player_id,
                            card_id: rect.card.id,
                        })?;
                        self.close = true;
                    }
                    SelectionOverlayBehaviour::Pick => {
                        self.client.send(ClientMessage::PickCard {
                            game_id: self.game_id,
                            player_id: self.player_id,
                            card_id: rect.card.id,
                        })?;
                        self.close = true;
                    }
                }
            }
        }

        Ok(None)
    }

    fn render(&mut self, _data: &mut GameData, ui: &mut Ui, painter: &Painter) -> anyhow::Result<()> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );

        let cw = card_width().unwrap_or(80.0) * 2.0;
        let ch = card_height().unwrap_or(112.0) * 2.0;
        let card_count = self.card_rects.len();
        let card_spacing = 20.0;
        let cards_area_width = card_count as f32 * cw + (card_count as f32 - 1.0) * card_spacing;
        let cards_start_x = (sw - cards_area_width) / 2.0;
        let cards_y = (sh - ch) / 2.0 + 30.0;

        painter.text(
            pos2(cards_start_x - 50.0, cards_y - 50.0),
            egui::Align2::LEFT_TOP,
            &self.prompt,
            egui::FontId::proportional(FONT_SIZE),
            Color32::WHITE,
        );

        for card_rect in &self.card_rects {
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
        }

        if self.behaviour == SelectionOverlayBehaviour::Preview {
            let close_button_pos = pos2(sw / 2.0 - 50.0, cards_y + ch + 20.0);
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
