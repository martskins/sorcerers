use sorcerers::networking;

pub mod game;
pub mod menu;

#[derive(Debug)]
pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
}

impl Scene {
    pub async fn render(&mut self, client: &mut networking::client::Client) {
        match self {
            Scene::Menu(menu) => menu.render(client).await,
            Scene::Game(game) => game.render(client).await,
        }
    }

    pub async fn process_input(
        &mut self,
        client: &mut networking::client::Client,
    ) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_input(client).await,
            Scene::Game(game) => game.process_input(client).await,
        }
    }
}
