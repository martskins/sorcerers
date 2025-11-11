use crate::card::{Card, CardType, CardZone};
use macroquad::prelude::*;

pub struct Avatar {
    pub id: uuid::Uuid,
    pub name: String,
    pub texture: Texture2D,
    pub rect: Option<Rect>,
    pub zone: CardZone,
    pub hovered: bool,
    pub selected: bool,
}

impl Avatar {
    pub async fn from_name(name: &str) -> Option<Self> {
        let texture: Texture2D = load_texture(&crate::card::get_image_path(name).as_str())
            .await
            .unwrap();
        Some(Avatar {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            texture,
            rect: None,
            zone: CardZone::None,
            hovered: false,
            selected: false,
        })
    }
}

impl Clone for Avatar {
    fn clone(&self) -> Self {
        Avatar {
            id: self.id.clone(),
            name: self.name.clone(),
            texture: self.texture.clone(),
            rect: self.rect.clone(),
            zone: self.zone.clone(),
            hovered: self.hovered,
            selected: self.selected,
        }
    }
}
