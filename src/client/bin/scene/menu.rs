use crate::{
    config::screen_rect,
    render::menu_skin,
    scene::{Scene, game::Game},
};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData};
use macroquad::{
    color::{ORANGE, WHITE},
    math::Vec2,
    text::draw_text,
    ui::{self, hash, root_ui},
};
use sorcerers::networking::message::ServerMessage;
use sorcerers::networking::{
    self,
    message::{ClientMessage, PreconDeck},
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
                if self.player_id.is_none() {
                    return Err(anyhow::anyhow!("Received GameStarted without a player_id"));
                }

                let player_id = self.player_id.expect("player_id should be set");
                let opponent_id = if player1 == &player_id {
                    player2.clone()
                } else {
                    player1.clone()
                };

                let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
                let sound_data = StaticSoundData::from_file("assets/sounds/game_start.mp3")?;
                manager.play(sound_data.clone())?;

                Ok(Some(Scene::Game(
                    Game::new(
                        game_id.clone(),
                        player_id,
                        opponent_id,
                        &player_id == player1,
                        cards.clone(),
                        self.client.clone(),
                    )
                    .await?,
                )))
            }
            _ => Ok(None),
        }
    }

    pub async fn render(&mut self) -> anyhow::Result<Option<Scene>> {
        if self.client.is_in_local_mode() {
            let message = "Warning: Running in local mode!";
            draw_text(&message, 10.0, 20.0, 24.0, ORANGE);
        }

        const FONT_SIZE: f32 = 24.0;
        if self.looking_for_match {
            let time = macroquad::time::get_time();
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let mut dots = ".".repeat(dot_count);
            dots += &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);

            let screen_rect = screen_rect()?;
            let text_dimensions = macroquad::text::measure_text(&message, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(&message, x, y, 32.0, WHITE);
        }

        Ok(None)
    }

    async fn render_deck_list(&mut self) -> anyhow::Result<Option<Scene>> {
        let button_size = Vec2::new(300.0, 60.0);
        let screen_w = macroquad::window::screen_width();
        let screen_h = macroquad::window::screen_height();

        let skin = menu_skin();
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
                self.client.send(ClientMessage::JoinQueue {
                    player_name: self.player_name.clone(),
                    player_id: self.player_id.expect("player id should be set").clone(),
                    deck: deck.clone(),
                })?;
                root_ui().pop_skin();
            }
        }

        if chose_deck {
            self.looking_for_match = true;
        }

        root_ui().pop_skin();
        Ok(None)
    }

    async fn render_lobby_form(&mut self) -> anyhow::Result<Option<Scene>> {
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

        let skin = menu_skin();
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
                self.client.send(ClientMessage::Connect)?;
            }
        }

        root_ui().pop_skin();
        Ok(None)
    }

    pub async fn process_input(&mut self) -> anyhow::Result<Option<Scene>> {
        if self.looking_for_match {
            return Ok(None);
        }

        if self.available_decks.is_empty() {
            self.render_lobby_form().await
        } else {
            self.render_deck_list().await
        }
    }
}
