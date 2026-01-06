mod client;
pub mod components;
mod config;
pub mod input;
mod render;
mod scene;
mod texture_cache;

use crate::{client::Client, texture_cache::TextureCache};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Sorcerers".to_owned(),
        high_dpi: true,
        // window_width: 1280,
        // window_height: 720,
        fullscreen: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
    TextureCache::init();

    let server_url: String = std::env::var("SORCERERS_SERVER_URL").unwrap_or("127.0.0.1:5000".to_string());
    let mut client = Client::new(&server_url)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    client.start(tx).unwrap();

    loop {
        if let Ok(msg) = rx.try_recv() {
            let new_scene = client.scene.process_message(&msg).await.unwrap();
            if let Some(new_scene) = new_scene {
                client.scene = new_scene;
            }
        }

        if let Some(scene) = client.step().await? {
            client.scene = scene;
        }

        next_frame().await;
    }
}
