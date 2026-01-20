use std::collections::HashMap;

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width},
    input::Mouse,
    render::{self, CardRect},
    scene::game::GameData,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{BLACK, Color, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{Rect, RectOffset, Vec2},
    shapes::draw_rectangle,
    ui,
    window::{screen_height, screen_width},
};
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
    pub async fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        card_previews: Vec<&CardData>,
        player_id: &PlayerId,
        prompt: String,
        action: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut textures = HashMap::with_capacity(card_previews.len());
        for defender in &card_previews {
            let texture = TextureCache::get_card_texture(&defender).await?;
            textures.insert(defender.id, texture);
        }

        let card_width = card_width()? * 1.2;
        let card_height = card_height()? * 1.2;
        let card_spacing = 20.0;

        let preview_y = (screen_height() / 3.0) - (card_height / 2.0);
        let defender_count = card_previews.len();
        let defenders_area_width = defender_count as f32 * card_width + ((defender_count as f32 - 1.0) * card_spacing);
        let defenders_start_x = (screen_width() - defenders_area_width) / 2.0;

        let mut rects = Vec::with_capacity(card_previews.len());
        for (idx, defender) in card_previews.iter().enumerate() {
            let x = defenders_start_x + idx as f32 * (card_width + card_spacing);
            rects.push(CardRect {
                rect: Rect::new(x, preview_y, card_width, card_height),
                card: (*defender).clone(),
                image: textures.get(&defender.id).unwrap().clone(),
                is_hovered: false,
                is_selected: false,
            });
        }

        Ok(Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt: prompt,
            player_id: player_id.clone(),
            visible: true,
            action,
        })
    }
}

#[async_trait::async_trait]
impl Component for ActionOverlay {
    async fn update(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        if is_mouse_button_released(MouseButton::Left) {
            Mouse::set_enabled(true)?;
        }

        Ok(())
    }

    async fn process_command(&mut self, _command: &ComponentCommand, _data: &mut GameData) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::ActionOverlay
    }

    async fn process_input(
        &mut self,
        _in_turn: bool,
        _data: &mut GameData,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let mut skin = ui::root_ui().default_skin();
        skin.label_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(WHITE)
            .build();
        skin.button_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(BLACK)
            .build();
        ui::root_ui().push_skin(&skin);

        if self.action.is_some() {
            let card_row_y = self
                .card_rects
                .iter()
                .map(|r| r.rect.y + r.rect.h)
                .collect::<Vec<f32>>()[0];
            let button_width = 150.0;
            let button_height = 40.0;
            let button_spacing = 40.0;
            let total_width = button_width * 2.0 + button_spacing;
            let button_x = (screen_width() - total_width) / 2.0;
            let button_y = card_row_y + 40.0;

            let yes_pressed = ui::widgets::Button::new("Yes")
                .position(Vec2::new(button_x, button_y))
                .size(Vec2::new(button_width, button_height))
                .ui(&mut ui::root_ui());

            let no_pressed = ui::widgets::Button::new("No")
                .position(Vec2::new(button_x + button_width + button_spacing, button_y))
                .size(Vec2::new(button_width, button_height))
                .ui(&mut ui::root_ui());

            if yes_pressed || no_pressed {
                self.client.send(ClientMessage::ResolveAction {
                    game_id: self.game_id.clone(),
                    player_id: self.player_id.clone(),
                    take_action: yes_pressed,
                })?;
                self.visible = false;
            }
        } else {
            let card_row_y = self
                .card_rects
                .iter()
                .map(|r| r.rect.y + r.rect.h)
                .collect::<Vec<f32>>()[0];
            let button_width = 150.0;
            let button_height = 40.0;
            let button_x = (screen_width() - button_width) / 2.0;
            let button_y = card_row_y + 40.0;

            let ok_pressed = ui::widgets::Button::new("Ok")
                .position(Vec2::new(button_x, button_y))
                .size(Vec2::new(button_width, button_height))
                .ui(&mut ui::root_ui());

            if ok_pressed {
                self.visible = false;
            }
        }

        ui::root_ui().pop_skin();
        Ok(None)
    }

    async fn render(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.8),
        );

        let window_style = ui::root_ui()
            .style_builder()
            .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
            .build();
        let label_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(WHITE)
            .build();
        let skin = ui::Skin {
            window_style,
            label_style,
            ..ui::root_ui().default_skin()
        };

        ui::root_ui().push_skin(&skin);

        let mut skin = ui::root_ui().default_skin();
        skin.button_style = ui::root_ui()
            .style_builder()
            .color(Color::new(0.0, 0.0, 0.0, 0.0))
            .build();
        skin.label_style = ui::root_ui()
            .style_builder()
            .text_color(WHITE)
            .font_size(FONT_SIZE as u16)
            .build();
        ui::root_ui().push_skin(&skin);

        let text_dims = macroquad::text::measure_text(&self.prompt, None, FONT_SIZE as u16, 1.0);
        let wrapped_text = render::wrap_text(&self.prompt, screen_width() - 20.0, FONT_SIZE as u16);
        render::multiline_label(
            &wrapped_text,
            Vec2::new((screen_width() / 2.0) - (text_dims.width / 2.0), 30.0),
            FONT_SIZE as u16,
            &mut ui::root_ui(),
        );

        if self.action.is_some() {
            let action_text = self.action.as_ref().unwrap();
            let action_dims = macroquad::text::measure_text(action_text, None, FONT_SIZE as u16, 1.0);
            let wrapped_text = render::wrap_text(action_text, screen_width() - 20.0, FONT_SIZE as u16);
            render::multiline_label(
                &wrapped_text,
                Vec2::new((screen_width() / 2.0) - (action_dims.width / 2.0), 70.0),
                FONT_SIZE as u16,
                &mut ui::root_ui(),
            );
        }

        for card_rect in &self.card_rects {
            render::draw_card(card_rect, card_rect.card.controller_id == self.player_id, false);
        }

        ui::root_ui().pop_skin();
        Ok(())
    }
}
