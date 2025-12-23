use macroquad::texture::Texture2D;
use sorcerers::card::RenderableCard;
use std::{
    collections::HashMap,
    path::Path,
    sync::{OnceLock, RwLock},
};

static TEXTURE_CACHE: OnceLock<RwLock<TextureCache>> = OnceLock::new();

#[derive(Debug)]
pub struct TextureCache {
    inner: HashMap<String, macroquad::texture::Texture2D>,
}

impl TextureCache {
    fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    pub fn init() {
        TEXTURE_CACHE.get_or_init(|| RwLock::new(TextureCache::new()));
    }

    pub async fn get_card_texture(card: &RenderableCard) -> Texture2D {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { TextureCache::texture_for_card(card).await })
    }

    #[allow(dead_code)]
    pub async fn get_texture(path: &str, name: &str) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(name) {
            return tex.clone();
        }

        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        let new_path = path.to_string();
        let texture = macroquad::texture::load_texture(&new_path).await.unwrap();
        cache.inner.insert(name.to_string(), texture.clone());
        texture
    }

    async fn texture_for_card(card: &RenderableCard) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(card.get_name()) {
            return tex.clone();
        }

        let path = format!("assets/images/cache/{}.png", card.get_name());
        if Path::new(&path).exists() {
            return TextureCache::get_card_image_from_disk(card.get_name(), &path)
                .await
                .unwrap();
        }

        TextureCache::download_card_image(card).await.unwrap()
    }

    async fn get_card_image_from_disk(name: &str, path: &str) -> anyhow::Result<Texture2D> {
        let texture = macroquad::texture::load_texture(path).await?;
        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        cache.inner.insert(name.to_string(), texture.clone());
        Ok(texture)
    }

    async fn download_card_image(card: &RenderableCard) -> anyhow::Result<Texture2D> {
        let set = card.get_edition().url_name();
        let name = card
            .get_name()
            .to_string()
            .to_lowercase()
            .replace(" ", "_")
            .replace("-", "_");
        let mut folder = "cards";
        if card.is_site() {
            folder = "rotated";
        }
        let mut after_card_name = "b";
        if card.is_token() {
            after_card_name = "bt";
        }

        let path = format!(
            "https://d27a44hjr9gen3.cloudfront.net/{}/{}-{}-{}-s.png",
            folder, set, name, after_card_name
        );
        let response = reqwest::get(&path).await?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!(
                "Failed to download image for {} on path {}: HTTP {}",
                name,
                path,
                response.status()
            ));
        }

        let bytes = response.bytes().await.unwrap();
        let texture = macroquad::texture::Texture2D::from_file_with_format(&bytes, None);
        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        cache.inner.insert(name.to_string(), texture.clone());

        let save_path = format!("assets/images/cache/{}.png", name);
        if let Err(e) = std::fs::write(&save_path, &bytes) {
            println!("Error saving image to disk: {}", e);
        }

        Ok(texture)
    }
}
