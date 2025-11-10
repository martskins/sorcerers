mod card;
mod deck;
mod game;
mod player;
mod playground;
mod render;
mod window;

use crate::window::*;
use game::Game;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Sorcerer".to_owned(),
        // fullscreen: true,
        window_height: SCREEN_HEIGHT as i32,
        window_width: SCREEN_WIDTH as i32,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::setup().await;
    game.start();

    loop {
        game.step().await;
        next_frame().await
    }
}
