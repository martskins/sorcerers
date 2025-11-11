use macroquad::{math::Vec2, ui::root_ui};
use sorcerers::networking::{self, Message};

use crate::scene::Scene;

#[derive(Debug)]
pub struct Menu {}

impl Menu {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn render(&mut self, client: &mut networking::client::Client) {
        root_ui().label(Vec2::new(20.0, 20.0), "Menu Scene");
    }

    pub async fn process_input(
        &mut self,
        client: &mut networking::client::Client,
    ) -> Option<Scene> {
        if root_ui().button(Vec2::new(100.0, 100.0), "Connect!") {
            client.send(Message::Connect).unwrap();
            let msg = client.recv().unwrap();
            match msg {
                Message::ConnectResponse { player_id } => {
                    return Some(Scene::Game(crate::scene::game::Game::new(player_id)));
                }
                _ => return None,
            }
        }

        None
    }
}
