use sorcerers::networking::message::ServerMessage;
use egui::{Context, Ui};

pub mod action_overlay;
pub mod combat_resolution_overlay;
pub mod game;
pub mod menu;
pub mod selection_overlay;

pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
}

impl Scene {
    pub fn render(&mut self, ui: &mut Ui, ctx: &Context) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.render(ui, ctx),
            Scene::Game(game) => game.render(ui, ctx),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        match self {
            Scene::Menu(menu) => menu.update(ctx),
            Scene::Game(game) => game.update(ctx),
        }
    }

    pub fn process_message(&mut self, message: &ServerMessage) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_message(message),
            Scene::Game(game) => game.process_message(message),
        }
    }

    pub fn process_input(&mut self, ctx: &Context) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_input(ctx),
            Scene::Game(game) => {
                game.process_input(ctx);
                None
            }
        }
    }
}
