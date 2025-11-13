use sorcerers::networking;

pub mod game;
pub mod menu;

#[derive(Debug)]
pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
}

impl Scene {
    pub async fn render(&mut self) -> anyhow::Result<()> {
        match self {
            Scene::Menu(menu) => menu.render().await,
            Scene::Game(game) => game.render().await,
        }
    }

    pub async fn update(&mut self) {
        match self {
            Scene::Menu(menu) => menu.update().await,
            Scene::Game(game) => game.update().await,
        }
    }

    pub async fn process_message(&mut self, message: networking::Message) -> anyhow::Result<()> {
        match self {
            Scene::Menu(menu) => menu.process_message(message).await,
            Scene::Game(game) => game.process_message(message).await,
        }
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_input().await,
            Scene::Game(game) => game.process_input().await,
        }
    }
}
