use egui::{Context, Ui};
use sorcerers::networking::message::ServerMessage;

pub mod action_overlay;
pub mod card_toast;
pub mod combat_resolution_overlay;
pub mod deck_builder;
pub mod game;
pub mod menu;
pub mod selection_overlay;

#[allow(clippy::large_enum_variant)]
pub enum Scene {
    Menu(menu::Menu),
    Game(game::Game),
    DeckBuilder(deck_builder::DeckBuilder),
}

impl Scene {
    pub fn render(&mut self, ui: &mut Ui) -> Option<Scene> {
        match self {
            Scene::Menu(menu) => menu.render(ui),
            Scene::Game(game) => game.render(ui),
            Scene::DeckBuilder(db) => db.render(ui),
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
}
