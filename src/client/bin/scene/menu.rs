use macroquad::{math::Vec2, ui::root_ui};
use sorcerers::networking::{self, Message};

use crate::scene::Scene;

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

    pub async fn process_message(&mut self, _msg: networking::Message) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        if root_ui().button(Vec2::new(100.0, 100.0), "Connect!") {
            self.client.send(Message::Connect).unwrap();
            let msg = self.client.recv().unwrap();
            if let Message::ConnectResponse { player_id } = msg {
                return Some(Scene::Game(crate::scene::game::Game::new(
                    player_id,
                    self.client.clone(),
                )));
            }
        }

        None
    }
}
