use macroquad::texture::{Image, Texture2D};
use sorcerers::card::Edition;
use std::{
    collections::HashMap,
    path::Path,
    sync::{OnceLock, RwLock},
};
use tokio_util::bytes::Bytes;

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

    pub async fn get_card_texture(name: &str, is_site: bool, edition: &Edition) -> Texture2D {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { TextureCache::texture_for_card(name, is_site, edition).await })
    }

    pub async fn get_texture(path: &str, name: &str, rotate: bool) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(name) {
            return tex.clone();
        }

        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        let new_path = path.to_string();
        let texture = macroquad::texture::load_texture(&new_path).await.unwrap();
        // if rotate {
        //     TextureCache::rotate_texture_clockwise(&mut texture);
        // }
        cache.inner.insert(name.to_string(), texture.clone());
        texture
    }

    async fn texture_for_card(name: &str, is_site: bool, edition: &Edition) -> Texture2D {
        if let Some(tex) = TEXTURE_CACHE.get().unwrap().read().unwrap().inner.get(name) {
            return tex.clone();
        }

        let path = format!("assets/images/cache/{}.png", name);
        if Path::new(&path).exists() {
            return TextureCache::get_card_image_from_disk(name, &path).await.unwrap();
        }

        TextureCache::download_card_image(name, is_site, edition).await.unwrap()
    }

    async fn get_card_image_from_disk(name: &str, path: &str) -> anyhow::Result<Texture2D> {
        let texture = macroquad::texture::load_texture(path).await?;
        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        cache.inner.insert(name.to_string(), texture.clone());
        Ok(texture)
    }

    async fn download_card_image(name: &str, is_site: bool, edition: &Edition) -> anyhow::Result<Texture2D> {
        let set = edition.url_name();
        let name = name.to_string().to_lowercase().replace(" ", "_").replace("-", "_");
        let mut folder = "cards";
        if is_site {
            folder = "rotated";
        }

        let mut path = format!(
            "https://d27a44hjr9gen3.cloudfront.net/{}/{}-{}-b-s.png",
            folder, set, name
        );
        if name == "rubble" {
            path = "https://d27a44hjr9gen3.cloudfront.net/rotated/alp-rubble-bt-s.png".to_string();
        }
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
        // if is_site {
        //     TextureCache::rotate_texture_clockwise(&mut texture);
        //     let rotated_image = texture.get_texture_data();
        //     let dyn_img = image::DynamicImage::ImageRgba8(
        //         image::RgbaImage::from_raw(
        //             rotated_image.width() as u32,
        //             rotated_image.height() as u32,
        //             rotated_image.bytes.to_vec(),
        //         )
        //         .unwrap(),
        //     );
        //
        //     let mut png_bytes: Vec<u8> = Vec::new();
        //     dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageOutputFormat::Png)?;
        //     bytes = Bytes::copy_from_slice(&png_bytes);
        // }

        let mut cache = TEXTURE_CACHE.get().unwrap().write().unwrap();
        cache.inner.insert(name.to_string(), texture.clone());

        let save_path = format!("assets/images/cache/{}.png", name);
        if let Err(e) = std::fs::write(&save_path, &bytes) {
            println!("Error saving image to disk: {}", e);
        }

        Ok(texture)
    }

    // fn rotate_texture_clockwise(texture: &mut Texture2D) {
    //     let image = texture.get_texture_data();
    //     let (w, h) = (image.width() as u32, image.height() as u32);
    //     let mut rotated = Image::gen_image_color(h.try_into().unwrap(), w.try_into().unwrap(), macroquad::color::WHITE);
    //
    //     for y in 0..h {
    //         for x in 0..w {
    //             let pixel = image.get_pixel(x, y);
    //             rotated.set_pixel(h - y - 1, x, pixel);
    //         }
    //     }
    //
    //     *texture = Texture2D::from_image(&rotated);
    // }
}
