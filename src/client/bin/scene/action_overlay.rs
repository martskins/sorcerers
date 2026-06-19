use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{CARD_ASPECT_RATIO, card_height, screen_rect},
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

#[derive(Debug)]
pub struct ActionOverlay {
    source_card_rect: Option<CardRect>,
    card_rects: Vec<CardRect>,
    prompt: String,
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    action: Option<String>,
    visible: bool,
}

impl ActionOverlay {
    fn portrait_card_size(scale: f32) -> egui::Vec2 {
        let height = card_height().unwrap_or(112.0) * scale;
        vec2(height * CARD_ASPECT_RATIO, height)
    }

    fn landscape_card_size(scale: f32) -> egui::Vec2 {
        let portrait = Self::portrait_card_size(scale);
        vec2(portrait.y, portrait.x)
    }

    pub fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        card_previews: Vec<&CardData>,
        source_card: Option<&CardData>,
        player_id: &PlayerId,
        prompt: String,
        action: Option<String>,
    ) -> Self {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let portrait = Self::portrait_card_size(1.0);
        let landscape = Self::landscape_card_size(1.0);
        let card_spacing = 20.0;
        let preview_y = (sh - portrait.y) / 2.0;
        let defenders_area_width = card_previews
            .iter()
            .map(|card| {
                if card.is_site() {
                    landscape.x
                } else {
                    portrait.x
                }
            })
            .sum::<f32>()
            + (card_previews.len().saturating_sub(1) as f32 * card_spacing);
        let defenders_start_x = (sw - defenders_area_width) / 2.0;

        let rects = card_previews
            .iter()
            .scan(defenders_start_x, |x, card| {
                let size = if card.is_site() { landscape } else { portrait };
                let rect = Rect::from_min_size(pos2(*x, preview_y), size);
                *x += size.x + card_spacing;
                Some((rect, *card))
            })
            .map(|(rect, card)| CardRect {
                rect,
                card: (*card).clone(),
                image: None,
                is_selected: false,
            })
            .collect();
        let source_size = Self::portrait_card_size(0.58);
        let source_card_rect = source_card.map(|card| CardRect {
            rect: Rect::from_min_size(pos2(0.0, 0.0), source_size),
            card: card.clone(),
            image: None,
            is_selected: false,
        });

        Self {
            client,
            game_id: *game_id,
            source_card_rect,
            card_rects: rects,
            prompt,
            player_id: *player_id,
            visible: true,
            action,
        }
    }
}

impl Component for ActionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        if let Some(source_card_rect) = &mut self.source_card_rect
            && source_card_rect.image.is_none()
        {
            source_card_rect.image =
                TextureCache::get_card_texture_blocking(&source_card_rect.card, ctx);
        }
        for card_rect in &mut self.card_rects {
            if card_rect.image.is_none() {
                card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
            }
        }
        Ok(())
    }

    fn process_command(
        &mut self,
        _command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::ActionOverlay
    }

    fn render(
        &mut self,
        _data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );

        let pad = 18.0;
        let gap = 18.0;
        let button_width = 132.0;
        let button_height = 38.0;
        let source_box = vec2(104.0, 104.0);
        let source_column_w = if self.source_card_rect.is_some() {
            source_box.x + gap
        } else {
            0.0
        };
        let modal_w = if self.source_card_rect.is_some() {
            620.0_f32
        } else {
            500.0_f32
        }
        .min(sw - 32.0);
        let text_w = modal_w - (pad * 2.0) - source_column_w;
        let prompt_galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout(
                self.prompt.clone(),
                egui::FontId::proportional(15.0),
                Color32::from_rgb(214, 224, 245),
                text_w,
            )
        });
        let action_galley = self.action.as_ref().map(|action_text| {
            ui.ctx().fonts_mut(|fonts| {
                fonts.layout(
                    action_text.clone(),
                    egui::FontId::proportional(16.0),
                    Color32::WHITE,
                    text_w,
                )
            })
        });
        let prompt_y_offset = if self.source_card_rect.is_some() { 44.0 } else { 32.0 };
        let text_h = prompt_y_offset
            + prompt_galley.size().y
            + action_galley
                .as_ref()
                .map(|galley| 12.0 + galley.size().y)
                .unwrap_or_default();
        let header_h = if self.source_card_rect.is_some() {
            source_box.y.max(text_h)
        } else {
            text_h
        };
        let preview_h = (sh * 0.30).clamp(150.0, 240.0);
        let card_spacing = 16.0;
        let mut preview_sizes = self
            .card_rects
            .iter()
            .map(|card_rect| {
                if card_rect.card.is_site() {
                    vec2(preview_h, preview_h * CARD_ASPECT_RATIO)
                } else {
                    vec2(preview_h * CARD_ASPECT_RATIO, preview_h)
                }
            })
            .collect::<Vec<_>>();
        let preview_row_w = preview_sizes.iter().map(|size| size.x).sum::<f32>()
            + card_spacing * preview_sizes.len().saturating_sub(1) as f32;
        if preview_row_w > modal_w - (pad * 2.0) && preview_row_w > 0.0 {
            let scale = (modal_w - (pad * 2.0)) / preview_row_w;
            for size in &mut preview_sizes {
                *size *= scale;
            }
        }
        let preview_row_h = preview_sizes
            .iter()
            .map(|size| size.y)
            .fold(0.0_f32, f32::max);
        let modal_h = pad + header_h + gap + preview_row_h + gap + button_height + pad;
        let modal_rect = Rect::from_min_size(
            pos2((sw - modal_w) / 2.0, (sh - modal_h) / 2.0),
            vec2(modal_w, modal_h),
        );
        painter.rect_filled(
            modal_rect,
            9.0,
            Color32::from_rgba_unmultiplied(7, 9, 18, 242),
        );
        painter.rect_stroke(
            modal_rect,
            9.0,
            egui::Stroke::new(1.0, Color32::from_rgb(83, 96, 128)),
            egui::StrokeKind::Outside,
        );

        let header_min = modal_rect.min + vec2(pad, pad);
        let mut text_x = header_min.x;
        if let Some(source_card_rect) = &self.source_card_rect {
            let source_rect = Rect::from_min_size(header_min, source_box);
            painter.rect_filled(source_rect, 5.0, Color32::from_rgb(24, 29, 42));
            if let Some(tex) = source_card_rect.image.as_ref() {
                let texture_size = tex.size_vec2();
                let scale = (source_rect.width() / texture_size.x)
                    .min(source_rect.height() / texture_size.y);
                let image_size = texture_size * scale;
                let image_rect = Rect::from_center_size(source_rect.center(), image_size);
                painter.image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            painter.rect_stroke(
                source_rect,
                5.0,
                egui::Stroke::new(1.0, Color32::from_rgb(58, 157, 190)),
                egui::StrokeKind::Outside,
            );
            text_x = source_rect.max.x + gap;
        }
        painter.text(
            pos2(text_x, header_min.y),
            egui::Align2::LEFT_TOP,
            self.source_card_rect
                .as_ref()
                .map(|source| source.card.name.as_str())
                .unwrap_or("Card revealed"),
            egui::FontId::proportional(18.0),
            Color32::from_rgb(244, 248, 255),
        );
        if self.source_card_rect.is_some() {
            painter.text(
                pos2(text_x, header_min.y + 23.0),
                egui::Align2::LEFT_TOP,
                "Triggered ability",
                egui::FontId::proportional(12.0),
                Color32::from_rgb(132, 168, 215),
            );
        }
        let prompt_y = header_min.y + prompt_y_offset;
        let prompt_h = prompt_galley.size().y;
        painter.galley(pos2(text_x, prompt_y), prompt_galley, Color32::WHITE);
        if let Some(action_galley) = action_galley {
            painter.galley(
                pos2(text_x, prompt_y + 12.0 + prompt_h),
                action_galley,
                Color32::WHITE,
            );
        }

        let preview_y = modal_rect.min.y + pad + header_h + gap;
        let preview_row_w = preview_sizes.iter().map(|size| size.x).sum::<f32>()
            + card_spacing * preview_sizes.len().saturating_sub(1) as f32;
        let mut preview_x = modal_rect.center().x - (preview_row_w / 2.0);
        for (card_rect, size) in self.card_rects.iter_mut().zip(preview_sizes.iter()) {
            card_rect.rect = Rect::from_min_size(pos2(preview_x, preview_y), *size);
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
            preview_x += size.x + card_spacing;
        }

        let button_y = modal_rect.max.y - pad - button_height;
        let mut should_close = false;
        if self.action.is_some() {
            let total_width = button_width * 2.0 + 16.0;
            let button_x = modal_rect.center().x - total_width / 2.0;
            let client = self.client.clone();
            let game_id = self.game_id;
            let player_id = self.player_id;
            let yes_rect =
                Rect::from_min_size(pos2(button_x, button_y), vec2(button_width, button_height));
            let no_rect = Rect::from_min_size(
                pos2(button_x + button_width + 16.0, button_y),
                vec2(button_width, button_height),
            );
            let yes = egui::Button::new(
                egui::RichText::new("Yes").size(18.0).color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(36, 51, 77));
            if ui.put(yes_rect, yes).clicked() {
                client
                    .send(ClientMessage::ResolveAction {
                        game_id,
                        player_id,
                        take_action: true,
                    })
                    .ok();
                should_close = true;
            }
            let no = egui::Button::new(
                egui::RichText::new("No").size(18.0).color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(36, 51, 77));
            if ui.put(no_rect, no).clicked() {
                client
                    .send(ClientMessage::ResolveAction {
                        game_id,
                        player_id,
                        take_action: false,
                    })
                    .ok();
                should_close = true;
            }
        } else {
            let button_x = modal_rect.center().x - button_width / 2.0;
            let ok_rect =
                Rect::from_min_size(pos2(button_x, button_y), vec2(button_width, button_height));
            let ok = egui::Button::new(
                egui::RichText::new("Ok").size(18.0).color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(36, 51, 77));
            if ui.put(ok_rect, ok).clicked() {
                should_close = true;
            }
        }

        if should_close {
            return Ok(Some(ComponentCommand::CloseOverlay));
        }
        Ok(None)
    }
}
