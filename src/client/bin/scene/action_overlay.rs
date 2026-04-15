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

const FONT_SIZE: f32 = 24.0;

#[derive(Debug)]
pub struct ActionOverlay {
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
        player_id: &PlayerId,
        prompt: String,
        action: Option<String>,
    ) -> Self {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let portrait = Self::portrait_card_size(1.2);
        let landscape = Self::landscape_card_size(1.2);
        let card_spacing = 20.0;
        let preview_y = (sh / 3.0) - (portrait.y / 2.0);
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
                is_hovered: false,
                is_selected: false,
            })
            .collect();

        Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt,
            player_id: player_id.clone(),
            visible: true,
            action,
        }
    }
}

impl Component for ActionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
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
    ) -> anyhow::Result<()> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );

        painter.text(
            pos2(sw / 2.0, 30.0),
            egui::Align2::CENTER_TOP,
            &self.prompt,
            egui::FontId::proportional(FONT_SIZE),
            Color32::WHITE,
        );

        if let Some(ref action_text) = self.action {
            painter.text(
                pos2(sw / 2.0, 70.0),
                egui::Align2::CENTER_TOP,
                action_text,
                egui::FontId::proportional(FONT_SIZE),
                Color32::WHITE,
            );
        }

        for card_rect in &self.card_rects {
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
        }

        let card_row_y = self
            .card_rects
            .iter()
            .map(|r| r.rect.max.y)
            .fold(0.0_f32, f32::max);
        let button_width = 150.0;
        let button_height = 40.0;
        let button_y = card_row_y + 40.0;

        if self.action.is_some() {
            let total_width = button_width * 2.0 + 40.0;
            let button_x = (sw - total_width) / 2.0;
            let client = self.client.clone();
            let game_id = self.game_id;
            let player_id = self.player_id;
            let mut set_invisible = false;

            egui::Area::new(egui::Id::new("action_overlay_btns"))
                .fixed_pos(pos2(button_x, button_y))
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        let yes = egui::Button::new(
                            egui::RichText::new("Yes").size(22.0).color(Color32::WHITE),
                        )
                        .min_size(vec2(button_width, button_height));
                        if ui.add(yes).clicked() {
                            client
                                .send(ClientMessage::ResolveAction {
                                    game_id,
                                    player_id,
                                    take_action: true,
                                })
                                .ok();
                            set_invisible = true;
                        }
                        ui.add_space(40.0);
                        let no = egui::Button::new(
                            egui::RichText::new("No").size(22.0).color(Color32::WHITE),
                        )
                        .min_size(vec2(button_width, button_height));
                        if ui.add(no).clicked() {
                            client
                                .send(ClientMessage::ResolveAction {
                                    game_id,
                                    player_id,
                                    take_action: false,
                                })
                                .ok();
                            set_invisible = true;
                        }
                    });
                });
            if set_invisible {
                self.visible = false;
            }
        } else {
            let button_x = (sw - button_width) / 2.0;
            let mut set_invisible = false;
            egui::Area::new(egui::Id::new("action_overlay_ok_btn"))
                .fixed_pos(pos2(button_x, button_y))
                .show(ui.ctx(), |ui| {
                    let ok = egui::Button::new(
                        egui::RichText::new("Ok").size(22.0).color(Color32::WHITE),
                    )
                    .min_size(vec2(button_width, button_height));
                    if ui.add(ok).clicked() {
                        set_invisible = true;
                    }
                });
            if set_invisible {
                self.visible = false;
            }
        }

        Ok(())
    }
}
