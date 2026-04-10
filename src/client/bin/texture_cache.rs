use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use sorcerers::card::CardData;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Mutex, OnceLock,
        mpsc::{Receiver, Sender, channel},
    },
};

static TEXTURE_CACHE: OnceLock<Mutex<TextureCache>> = OnceLock::new();

pub struct TextureCache {
    handles: HashMap<String, TextureHandle>,
    pending: HashMap<String, ()>,
    tx: Sender<(String, ColorImage)>,
    rx: Receiver<(String, ColorImage)>,
}

impl TextureCache {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            handles: HashMap::new(),
            pending: HashMap::new(),
            tx,
            rx,
        }
    }

    pub fn init() {
        TEXTURE_CACHE.get_or_init(|| Mutex::new(TextureCache::new()));
    }

    fn url_to_filename(url: &str) -> String {
        url.replace("://", "_").replace("/", "_")
    }

    pub fn flush_blocking(ctx: &Context) {
        if let Some(cache) = TEXTURE_CACHE.get() {
            if let Ok(mut cache_w) = cache.lock() {
                while let Ok((key, image)) = cache_w.rx.try_recv() {
                    let handle = ctx.load_texture(&key, image, TextureOptions::default());
                    cache_w.handles.insert(key.clone(), handle);
                    cache_w.pending.remove(&key);
                }
            }
        }
    }

    pub fn get_texture_blocking(path: &str, ctx: &Context) -> Option<TextureHandle> {
        if let Some(cache) = TEXTURE_CACHE.get() {
            if let Ok(mut cache_w) = cache.lock() {
                while let Ok((key, image)) = cache_w.rx.try_recv() {
                    let handle = ctx.load_texture(&key, image, TextureOptions::default());
                    cache_w.handles.insert(key.clone(), handle);
                    cache_w.pending.remove(&key);
                }
                if let Some(handle) = cache_w.handles.get(path) {
                    return Some(handle.clone());
                }
                if !cache_w.pending.contains_key(path) {
                    cache_w.pending.insert(path.to_string(), ());
                    let tx = cache_w.tx.clone();
                    let path_owned = path.to_string();
                    std::thread::spawn(move || {
                        if let Ok(bytes) = std::fs::read(&path_owned) {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                let size = [img.width() as usize, img.height() as usize];
                                let rgba = img.to_rgba8();
                                let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
                                let _ = tx.send((path_owned, color_image));
                            }
                        }
                    });
                }
            }
        }
        None
    }

    pub fn get_card_texture_blocking(card: &CardData, ctx: &Context) -> Option<TextureHandle> {
        if let Some(cache) = TEXTURE_CACHE.get() {
            if let Ok(mut cache_w) = cache.lock() {
                while let Ok((key, image)) = cache_w.rx.try_recv() {
                    let handle = ctx.load_texture(&key, image, TextureOptions::default());
                    cache_w.handles.insert(key.clone(), handle);
                    cache_w.pending.remove(&key);
                }
                if let Some(handle) = cache_w.handles.get(card.get_name()) {
                    return Some(handle.clone());
                }
                if cache_w.pending.contains_key(card.get_name()) {
                    return None;
                }
                let filename = Self::url_to_filename(&card.image_path);
                let disk_path = format!("assets/images/cache/{}", filename);
                cache_w.pending.insert(card.get_name().to_string(), ());
                let tx = cache_w.tx.clone();
                let name = card.get_name().to_string();
                let image_url = card.image_path.clone();
                let save_path = disk_path.clone();
                if Path::new(&disk_path).exists() {
                    std::thread::spawn(move || {
                        if let Ok(bytes) = std::fs::read(&save_path) {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                let size = [img.width() as usize, img.height() as usize];
                                let rgba = img.to_rgba8();
                                let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
                                let _ = tx.send((name, color_image));
                            }
                        }
                    });
                } else {
                    std::thread::spawn(move || match reqwest::blocking::get(&image_url) {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(bytes) = resp.bytes() {
                                let _ = std::fs::create_dir_all("assets/images/cache");
                                let _ = std::fs::write(&save_path, &bytes);
                                if let Ok(img) = image::load_from_memory(&bytes) {
                                    let size = [img.width() as usize, img.height() as usize];
                                    let rgba = img.to_rgba8();
                                    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
                                    let _ = tx.send((name, color_image));
                                }
                            }
                        }
                        _ => eprintln!("Failed to download image for {} in {}", name, image_url),
                    });
                }
            }
        }
        None
    }
}
