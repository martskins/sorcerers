use macroquad::texture::Texture2D;
use sorcerers::card::CardData;
use std::{collections::HashMap, path::Path, sync::OnceLock};
use tokio::sync::RwLock;

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

    pub async fn get_card_texture(card: &CardData) -> anyhow::Result<Texture2D> {
        if let Some(tex) = TEXTURE_CACHE
            .get()
            .ok_or(anyhow::anyhow!("failed to get texture cache reference"))?
            .read()
            .await
            .inner
            .get(card.get_name())
        {
            return Ok(tex.clone());
        }

        let path = format!("assets/images/cache/{}.png", card.get_name());
        if Path::new(&path).exists() {
            return TextureCache::get_card_image_from_disk(card.get_name(), &path).await;
        }

        TextureCache::download_card_image(card).await
    }

    #[allow(dead_code)]
    pub async fn load_cache(cards: &[CardData]) -> anyhow::Result<()> {
        for card in cards {
            _ = TextureCache::get_card_texture(card).await?;
        }

        Ok(())
    }

    pub async fn get_texture(path: &str) -> anyhow::Result<Texture2D> {
        if let Some(tex) = TEXTURE_CACHE
            .get()
            .ok_or(anyhow::anyhow!("failed to get texture cache reference"))?
            .read()
            .await
            .inner
            .get(path)
        {
            return Ok(tex.clone());
        }

        let mut cache = TEXTURE_CACHE
            .get()
            .ok_or(anyhow::anyhow!("failed to get texture cache reference"))?
            .write()
            .await;
        let new_path = path.to_string();
        let texture = macroquad::texture::load_texture(&new_path).await?;
        cache.inner.insert(path.to_string(), texture.clone());
        Ok(texture)
    }

    async fn get_card_image_from_disk(name: &str, path: &str) -> anyhow::Result<Texture2D> {
        let texture = macroquad::texture::load_texture(path).await?;
        let mut cache = TEXTURE_CACHE
            .get()
            .ok_or(anyhow::anyhow!("failed to get texture cache reference"))?
            .write()
            .await;
        cache.inner.insert(name.to_string(), texture.clone());
        Ok(texture)
    }

    async fn download_card_image(card: &CardData) -> anyhow::Result<Texture2D> {
        let set = card.get_edition().url_name();
        let card_name = card.get_name();
        let name_for_url = card
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
            folder, set, name_for_url, after_card_name
        );

        println!("Downloading image for {} from {}", card.get_name(), path);
        let response = reqwest::blocking::get(&path)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!(
                "Failed to download image for {} on path {}: HTTP {}",
                name_for_url,
                path,
                response.status()
            ));
        }

        let bytes = response.bytes()?;
        let texture = macroquad::texture::Texture2D::from_file_with_format(&bytes, None);
        let mut cache = TEXTURE_CACHE
            .get()
            .ok_or(anyhow::anyhow!("failed to get texture cache reference"))?
            .write()
            .await;
        cache.inner.insert(name_for_url.to_string(), texture.clone());

        let save_path = format!("assets/images/cache/{}.png", card_name);
        if let Err(e) = std::fs::write(&save_path, &bytes) {
            println!("Error saving image to disk: {}", e);
        }

        Ok(texture)
    }
}
