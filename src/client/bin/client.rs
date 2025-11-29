use crate::scene::Scene;
use crate::{config::SCREEN_RECT, scene::menu::Menu};
use macroquad::prelude::*;
use sorcerers::networking;
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct Client {
    scene: Arc<Mutex<Scene>>,
    client: networking::client::Client,
}

impl Client {
    pub fn new() -> anyhow::Result<Self> {
        let client = networking::client::Client::new("127.0.0.1:8080")?;
        let scene = Scene::Menu(Menu::new(client.clone()));
        let scene = Arc::new(Mutex::new(scene));

        let rect = Rect::new(0.0, 0.0, screen_width(), screen_height());
        SCREEN_RECT.get_or_init(|| RwLock::new(rect));
        Ok(Client { scene, client })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let receiver = self.client.clone();
        let scene = Arc::clone(&self.scene);
        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                loop {
                    let msg = receiver.recv().unwrap();
                    match msg {
                        _ => {
                            scene.lock().unwrap().process_message(msg.clone()).await.unwrap();
                        }
                    }
                }
            });
        });

        Ok(())
    }

    pub async fn step(&mut self) -> anyhow::Result<()> {
        self.process_input().await;
        self.update().await?;
        self.render().await?;
        Ok(())
    }

    async fn update(&mut self) -> anyhow::Result<()> {
        let scene = &mut self.scene.lock().unwrap();
        scene.update().await?;
        let current_screen = SCREEN_RECT.get().unwrap().read().unwrap();
        if current_screen.w != screen_width() || current_screen.h != screen_height() {
            SCREEN_RECT.get().unwrap().write().unwrap().w = screen_width();
            SCREEN_RECT.get().unwrap().write().unwrap().h = screen_height();
        }
        Ok(())
    }

    async fn render(&mut self) -> anyhow::Result<()> {
        clear_background(BLACK);
        let scene = &mut *self.scene.lock().unwrap();
        scene.render().await
    }

    async fn process_input(&mut self) {
        let current_scene = &mut *self.scene.lock().unwrap();
        let new_scene = current_scene.process_input().await;
        if let Some(scene) = new_scene {
            *current_scene = scene;
        }
    }
}
