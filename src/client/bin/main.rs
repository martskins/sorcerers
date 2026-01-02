mod client;
pub mod components;
mod config;
mod render;
mod scene;
mod texture_cache;

use crate::{client::Client, texture_cache::TextureCache};
use macroquad::prelude::*;
use std::sync::{LazyLock, Mutex};

// CLICK_ENABLED is set to false whenever a Button is click to prevent the release of the mouse
// button from triggering other actions in the same frame. This happens because buttons in
// macroquad respond to mouse button presses and our game mostly responds to mouse button
// releases, so a single click can trigger two actions.
static CLICK_ENABLED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(true));

pub fn set_clicks_enabled(enabled: bool) {
    let mut click_enabled = CLICK_ENABLED.lock().unwrap();
    *click_enabled = enabled;
}

pub fn clicks_enabled() -> bool {
    let click_enabled = CLICK_ENABLED.lock().unwrap();
    *click_enabled
}

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
