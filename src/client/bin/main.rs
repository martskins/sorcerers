mod client;
mod config;
mod render;
mod scene;
mod texture_cache;

use crate::{client::Client, texture_cache::TextureCache};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Sorcerer".to_owned(),
        // fullscreen: true,
        window_width: 1024,
        window_height: 768,
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
