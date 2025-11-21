use macroquad::texture::{Image, Texture2D};
use sorcerers::card::Card;
use std::{
    collections::HashMap,
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

    pub async fn get_card_texture(card: &Card) -> Texture2D {
        let rotate = card.is_site();
        TextureCache::get_texture(&card.get_image(), rotate).await
    }

    pub async fn get_texture(path: &str, rotate: bool) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(path) {
            return tex.clone();
        }

        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        let new_path = path.to_string();
        let mut texture = macroquad::texture::load_texture(&new_path).await.unwrap();
        if rotate {
            TextureCache::rotate_texture_clockwise(&mut texture);
        }
        cache.inner.insert(path.to_string(), texture.clone());
        texture
    }

    fn rotate_texture_clockwise(texture: &mut Texture2D) {
        let image = texture.get_texture_data();
        let (w, h) = (image.width() as u32, image.height() as u32);
        let mut rotated = Image::gen_image_color(h.try_into().unwrap(), w.try_into().unwrap(), macroquad::color::WHITE);

        for y in 0..h {
            for x in 0..w {
                let pixel = image.get_pixel(x, y);
                rotated.set_pixel(h - y - 1, x, pixel);
            }
        }

        *texture = Texture2D::from_image(&rotated);
    }
}
