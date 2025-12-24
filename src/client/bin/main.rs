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
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
    TextureCache::init();

    let mut client = Client::new().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    client.start(tx).unwrap();

    loop {
        if let Ok(msg) = rx.try_recv() {
            let new_scene = client.scene.process_message(&msg).await.unwrap();
            if let Some(new_scene) = new_scene {
                client.scene = new_scene;
            }
        }

        client.step().await.unwrap();
        next_frame().await;
    }
}
