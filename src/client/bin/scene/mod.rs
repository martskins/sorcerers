use sorcerers::networking::message::ServerMessage;

pub mod combat_resolution_overlay;
pub mod game;
pub mod menu;
pub mod selection_overlay;

pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
}

impl Scene {
    pub async fn render(&mut self) -> anyhow::Result<Option<Scene>> {
        match self {
            Scene::Menu(menu) => menu.render().await,
            Scene::Game(game) => game.render().await,
        }
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        match self {
            Scene::Menu(menu) => menu.update().await,
            Scene::Game(game) => game.update().await,
        }
    }

    pub async fn process_message(&mut self, message: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match self {
            Scene::Menu(menu) => menu.process_message(message).await,
            Scene::Game(game) => game.process_message(message).await,
        }
    }

    pub async fn process_input(&mut self) -> anyhow::Result<Option<Scene>> {
        match self {
            Scene::Menu(menu) => menu.process_input().await,
            Scene::Game(game) => {
                game.process_input().await?;
                Ok(None)
            }
        }
    }
}
