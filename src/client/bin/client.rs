use crate::scene::Scene;
use crate::{config::SCREEN_RECT, scene::menu::Menu};
use macroquad::prelude::*;
use sorcerers::networking;
use sorcerers::networking::message::{Message, ServerMessage};
use std::sync::RwLock;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

pub struct Client {
    pub scene: Scene,
    client: networking::client::Client,
}

impl Client {
    pub fn new(server_url: &str) -> anyhow::Result<Self> {
        let client = networking::client::Client::connect(server_url)?;
        let scene = Scene::Menu(Menu::new(client.clone()));

        let rect = Rect::new(0.0, 0.0, screen_width(), screen_height());
        SCREEN_RECT.get_or_init(|| RwLock::new(rect));
        Ok(Client { scene, client })
    }

    pub fn start(&self, sender: UnboundedSender<ServerMessage>) -> anyhow::Result<()> {
        let receiver = self.client.clone();
        std::thread::spawn(|| {
            let rt = Runtime::new().expect("runtime to be created");
            rt.block_on(async move {
                loop {
                    match receiver.recv().expect("message should be received") {
                        Some(Message::ServerMessage(msg)) => {
                            sender.send(msg).expect("message should be sent");
                        }
                        _ => {}
                    }
                }
            });
        });

        Ok(())
    }

    pub async fn step(&mut self) -> anyhow::Result<Option<Scene>> {
        self.process_input().await?;
        self.update().await?;
        Ok(self.render().await?)
    }

    fn dimensions_changed(&self) -> anyhow::Result<bool> {
        let dimensions = SCREEN_RECT
            .get()
            .ok_or(anyhow::anyhow!("failed to get SCREEN_RECT reference"))?;
        let current_screen = dimensions
            .read()
            .map_err(|e| anyhow::anyhow!("failed to lock for write: {}", e))?;
        Ok(current_screen.w != screen_width() || current_screen.h != screen_height())
    }

    async fn update(&mut self) -> anyhow::Result<()> {
        if self.dimensions_changed()? {
            let mut dimensions = SCREEN_RECT
                .get()
                .ok_or(anyhow::anyhow!("failed to get SCREEN_RECT reference"))?
                .write()
                .map_err(|e| anyhow::anyhow!("failed to lock for write: {}", e))?;
            dimensions.w = screen_width();
            dimensions.h = screen_height();
        }

        self.scene.update().await?;
        Ok(())
    }

    async fn render(&mut self) -> anyhow::Result<Option<Scene>> {
        clear_background(BLACK);
        self.scene.render().await
    }

    async fn process_input(&mut self) -> anyhow::Result<()> {
        let new_scene = self.scene.process_input().await?;
        if let Some(scene) = new_scene {
            self.scene = scene;
        }

        Ok(())
    }
}
