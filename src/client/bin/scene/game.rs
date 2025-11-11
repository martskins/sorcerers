use macroquad::{math::Vec2, ui::root_ui};
use sorcerers::{card::Card, networking};

use crate::scene::Scene;

#[derive(Debug)]
pub struct Game {
    pub player_id: uuid::Uuid,
    pub cards: Vec<Card>,
}

impl Game {
    pub fn new(player_id: uuid::Uuid) -> Self {
        Self {
            player_id,
            cards: vec![],
        }
    }

    pub async fn render(&mut self, client: &mut networking::client::Client) {
        root_ui().label(Vec2::new(20.0, 20.0), "Game Scene");
    }

    pub async fn process_input(
        &mut self,
        client: &mut networking::client::Client,
    ) -> Option<Scene> {
        None
    }
}
