use crate::scene::{Scene, game::Game};
use macroquad::{math::Vec2, ui::root_ui};
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
        if root_ui().button(Vec2::new(100.0, 100.0), "Connect!") {
            self.client.send(ClientMessage::Connect).unwrap();
        }

        None
    }
}
