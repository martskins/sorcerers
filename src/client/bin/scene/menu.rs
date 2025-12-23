use crate::scene::{Scene, game::Game};
use macroquad::{
    color::WHITE,
    math::Vec2,
    ui::{self, root_ui},
};
use sorcerers::networking::{
    self,
    message::{ClientMessage, ServerMessage},
};

#[derive(Debug)]
pub struct Menu {
    client: networking::client::Client,
}

impl Menu {
    pub fn new(client: networking::client::Client) -> Self {
        Self { client }
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        root_ui().label(Vec2::new(20.0, 20.0), "Menu Scene");
        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn process_message(&mut self, msg: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match msg {
            ServerMessage::ConnectResponse { player_id } => {
                Ok(Some(Scene::Game(Game::new(player_id.clone(), self.client.clone()))))
            }
            _ => Ok(None),
        }
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        let button_size = Vec2::new(300.0, 60.0);
        let screen_w = macroquad::window::screen_width();
        let screen_h = macroquad::window::screen_height();
        let button_pos = Vec2::new(
            screen_w / 2.0 - button_size.x / 2.0,
            screen_h / 2.0 - button_size.y / 2.0,
        );

        // Style the button to be more prominent
        let button_style = root_ui()
            .style_builder()
            .font_size(32)
            .text_color(WHITE)
            .text_color_hovered(WHITE)
            .text_color_clicked(WHITE)
            .color(macroquad::color::Color::from_rgba(30, 144, 255, 255)) // DodgerBlue
            .color_hovered(macroquad::color::Color::from_rgba(65, 105, 225, 255)) // RoyalBlue
            .color_clicked(macroquad::color::Color::from_rgba(25, 25, 112, 255)) // MidnightBlue
            .build();

        let skin = ui::Skin {
            button_style,
            ..root_ui().default_skin()
        };

        root_ui().push_skin(&skin);

        let clicked = ui::widgets::Button::new("Search for Match")
            .position(button_pos)
            .size(button_size)
            .ui(&mut ui::root_ui());
        if clicked {
            self.client.send(ClientMessage::Connect).unwrap();
        }

        root_ui().pop_skin();
        None
    }
}
