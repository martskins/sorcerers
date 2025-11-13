use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use macroquad::texture::Texture2D;

static TEXTURE_CACHE: OnceLock<RwLock<TextureCache>> = OnceLock::new();

#[derive(Debug)]
pub struct TextureCache {
    inner: HashMap<String, macroquad::texture::Texture2D>,
}

impl TextureCache {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn init() {
        TEXTURE_CACHE.get_or_init(|| RwLock::new(TextureCache::new()));
    }

    pub async fn get_texture(path: &str) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(path) {
            tex.clone()
        } else {
            let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
            let new_path = path.to_string();
            // if path.starts_with("http") {
            //     println!("Downloading texture from URL: {}", path);
            //     let img_bytes = reqwest::blocking::get(path).unwrap().bytes().unwrap();
            //     new_path = format!(
            //         "./assets/images/cache/{}",
            //         path.split('/')
            //             .last()
            //             .unwrap_or("default_texture.png")
            //             .split('?')
            //             .next()
            //             .unwrap()
            //     );
            //     let img = image::load_from_memory(&img_bytes).unwrap();
            //     image::save_buffer(
            //         &new_path,
            //         &img_bytes,
            //         img.width(),
            //         img.height(),
            //         img.color(),
            //     )
            //     .unwrap();
            // }
            let image = macroquad::texture::load_texture(&new_path).await.unwrap();
            cache.inner.insert(path.to_string(), image.clone());
            image
        }
    }
}
