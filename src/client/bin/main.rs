mod client;
mod config;
mod render;
mod scene;
mod texture_cache;

use crate::{client::Client, config::*, texture_cache::TextureCache};
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
async fn main() -> anyhow::Result<()> {
    let mut client = Client::new().unwrap();
    client.start().unwrap();

    TextureCache::init();

    loop {
        client.step().await.unwrap();
        next_frame().await
    }
}
