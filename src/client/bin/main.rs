mod client;
mod config;
mod scene;

use crate::{client::Client, config::*};
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
    let mut client = Client::new()?;
    client.start().unwrap();

    loop {
        client.step().await?;
        next_frame().await
    }
}
