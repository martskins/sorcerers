use egui::{Context, Ui};
use sorcerers::networking::message::ServerMessage;

pub mod action_overlay;
pub mod card_toast;
pub mod combat_resolution_overlay;
pub mod deck_builder;
pub mod game;
pub mod menu;
pub mod selection_overlay;

pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
    DeckBuilder(deck_builder::DeckBuilder),
}

impl Scene {
    pub fn render(&mut self, ui: &mut Ui, ctx: &Context) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.render(ui, ctx),
            Scene::Game(game) => game.render(ui, ctx),
            Scene::DeckBuilder(db) => db.render(ui, ctx),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        match self {
            Scene::Menu(menu) => menu.update(ctx),
            Scene::Game(game) => game.update(ctx),
            Scene::DeckBuilder(_) => {}
        }
    }

    pub fn process_message(&mut self, message: &ServerMessage) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_message(message),
            Scene::Game(game) => game.process_message(message),
            Scene::DeckBuilder(_) => None,
        }
    }

    pub fn process_input(&mut self, ctx: &Context) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.process_input(ctx),
            Scene::Game(game) => {
                game.process_input(ctx);
                None
            }
            Scene::DeckBuilder(db) => db.process_input(ctx),
        }
    }
}
