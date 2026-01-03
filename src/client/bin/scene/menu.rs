use crate::{
    config::screen_rect,
    scene::{Scene, game::Game},
};
use macroquad::{
    color::WHITE,
    math::Vec2,
    text::draw_text,
    ui::{self, hash, root_ui},
};
use sorcerers::networking::{
    self,
    message::{ClientMessage, PreconDeck, ServerMessage},
};

#[derive(Debug)]
pub struct Menu {
    client: networking::client::Client,
    player_id: Option<uuid::Uuid>,
    available_decks: Vec<PreconDeck>,
    looking_for_match: bool,
    shake_input_until: Option<chrono::DateTime<chrono::Utc>>,
    player_name: String,
}

impl Menu {
    pub fn new(client: networking::client::Client) -> Self {
        Self {
            client,
            player_id: None,
            available_decks: vec![],
            looking_for_match: false,
            shake_input_until: None,
            player_name: String::new(),
        }
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        const FONT_SIZE: f32 = 24.0;
        if self.looking_for_match {
            let time = macroquad::time::get_time();
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let mut dots = ".".repeat(dot_count);
            dots += &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);

            let screen_rect = screen_rect();
            let text_dimensions = macroquad::text::measure_text(&message, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(&message, x, y, 32.0, WHITE);
        }

        Ok(())
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn process_message(&mut self, msg: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match msg {
            ServerMessage::ConnectResponse {
                player_id,
                available_decks,
            } => {
                self.available_decks = available_decks.clone();
                self.player_id = Some(player_id.clone());
                Ok(None)
            }
            ServerMessage::GameStarted {
                player1,
                player2,
                game_id,
                cards,
            } => {
                let opponent_id = if player1 == &self.player_id.unwrap() {
                    player2.clone()
                } else {
                    player1.clone()
                };

                let player_id = self.player_id.unwrap();
                Ok(Some(Scene::Game(Game::new(
                    game_id.clone(),
                    player_id,
                    opponent_id,
                    &player_id == player1,
                    cards.clone(),
                    self.client.clone(),
                ))))
            }
            _ => Ok(None),
        }
    }

    pub async fn pick_deck_scene(&mut self) -> Option<Scene> {
        let button_size = Vec2::new(300.0, 60.0);
        let screen_w = macroquad::window::screen_width();
        let screen_h = macroquad::window::screen_height();
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

        let spacing = 20.0;
        let total_height =
            self.available_decks.len() as f32 * button_size.y + (self.available_decks.len() as f32 - 1.0) * spacing;
        let start_y = screen_h / 2.0 - total_height / 2.0;

        let mut chose_deck = false;
        for (i, deck) in self.available_decks.iter().enumerate() {
            let button_pos = Vec2::new(
                screen_w / 2.0 - button_size.x / 2.0,
                start_y + i as f32 * (button_size.y + spacing),
            );
            let clicked = ui::widgets::Button::new(deck.name())
                .position(button_pos)
                .size(button_size)
                .ui(&mut ui::root_ui());
            if clicked {
                chose_deck = true;
                self.client
                    .send(ClientMessage::JoinQueue {
                        player_name: self.player_name.clone(),
                        player_id: self.player_id.unwrap().clone(),
                        deck: deck.clone(),
                    })
                    .unwrap();
                root_ui().pop_skin();
            }
        }

        if chose_deck {
            self.looking_for_match = true;
        }

        root_ui().pop_skin();
        None
    }

    pub async fn start_scene(&mut self) -> Option<Scene> {
        let button_size = Vec2::new(300.0, 60.0);
        let screen_w = macroquad::window::screen_width();
        let screen_h = macroquad::window::screen_height();
        let button_pos = Vec2::new(
            screen_w / 2.0 - button_size.x / 2.0,
            screen_h / 2.0 - button_size.y / 2.0 + 70.0,
        );
        let mut input_pos = Vec2::new(
            screen_w / 2.0 - button_size.x / 2.0,
            screen_h / 2.0 - button_size.y / 2.0,
        );

        if let Some(shake_until) = self.shake_input_until {
            let now = chrono::Utc::now();
            if now < shake_until {
                let elapsed = (shake_until - now).num_milliseconds() as f32;
                let shake_magnitude = 3.0 * (elapsed / 300.0);
                let shake_x = (macroquad::rand::gen_range(-1.0, 1.0)) * shake_magnitude;
                input_pos.x += shake_x;
            } else {
                self.shake_input_until = None;
            }
        }

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
        let editbox_style = root_ui().style_builder().font_size(30).build();
        let label_style = root_ui().style_builder().font_size(30).text_color(WHITE).build();

        let skin = ui::Skin {
            button_style,
            editbox_style,
            label_style,
            ..root_ui().default_skin()
        };

        root_ui().push_skin(&skin);

        ui::widgets::Label::new("Enter your name")
            .position(Vec2::new(input_pos.x, input_pos.y - 30.0))
            .ui(&mut ui::root_ui());

        ui::widgets::InputText::new(hash!())
            .position(input_pos)
            .size(button_size)
            .margin(Vec2::new(0.0, 10.0))
            .ui(&mut ui::root_ui(), &mut self.player_name);

        let clicked = ui::widgets::Button::new("Search for Match")
            .position(button_pos)
            .size(button_size)
            .ui(&mut ui::root_ui());
        if clicked {
            if self.player_name.is_empty() {
                self.shake_input_until = Some(chrono::Utc::now() + chrono::Duration::milliseconds(500));
            } else {
                self.client.send(ClientMessage::Connect).unwrap();
            }
        }

        root_ui().pop_skin();
        None
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        if self.looking_for_match {
            return None;
        }

        if self.available_decks.is_empty() {
            self.start_scene().await
        } else {
            self.pick_deck_scene().await
        }
    }
}
