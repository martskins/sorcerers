use macroquad::{math::Vec2, prelude::Rect, texture::Texture2D};

use crate::window::{CARD_HEIGHT, CARD_WIDTH};

pub(crate) mod avatar;
pub(crate) mod database;
pub(crate) mod site;
pub(crate) mod spell;

#[derive(PartialEq, Clone)]
pub enum CardZone {
    None,
    Hand,
    Realm,
    Deck,
    DiscardPile,
}

pub enum Card {
    Spell(spell::Spell),
    Site(site::Site),
    Avatar(avatar::Avatar),
}

impl Card {
    pub fn get_name(&self) -> &str {
        match self {
            Card::Spell(spell) => &spell.name,
            Card::Site(site) => &site.name,
            Card::Avatar(avatar) => &avatar.name,
        }
    }

    pub fn get_type(&self) -> CardType {
        match self {
            Card::Spell(_) => CardType::Spell,
            Card::Site(_) => CardType::Site,
            Card::Avatar(_) => CardType::Avatar,
        }
    }

    pub fn get_texture(&self) -> &Texture2D {
        match self {
            Card::Spell(spell) => &spell.texture,
            Card::Site(site) => &site.texture,
            Card::Avatar(avatar) => &avatar.texture,
        }
    }

    pub fn get_dimensions(&self) -> Vec2 {
        match self {
            Card::Site(_) => Vec2::new(CARD_HEIGHT, CARD_WIDTH),
            _ => Vec2::new(CARD_WIDTH, CARD_HEIGHT),
        }
    }

    pub fn get_zone(&self) -> &CardZone {
        match self {
            Card::Spell(spell) => &spell.zone,
            Card::Site(site) => &site.zone,
            Card::Avatar(avatar) => &avatar.zone,
        }
    }

    pub fn get_rect(&self) -> &Option<Rect> {
        match self {
            Card::Spell(spell) => &spell.rect,
            Card::Site(site) => &site.rect,
            Card::Avatar(avatar) => &avatar.rect,
        }
    }

    pub fn get_is_hovered(&self) -> bool {
        match self {
            Card::Spell(spell) => spell.hovered,
            Card::Site(site) => site.hovered,
            Card::Avatar(avatar) => avatar.hovered,
        }
    }

    pub fn get_is_selected(&self) -> bool {
        match self {
            Card::Spell(spell) => spell.selected,
            Card::Site(site) => site.selected,
            Card::Avatar(avatar) => avatar.selected,
        }
    }

    pub fn set_zone(&mut self, zone: CardZone) {
        match self {
            Card::Spell(spell) => spell.zone = zone,
            Card::Site(site) => site.zone = zone,
            Card::Avatar(avatar) => avatar.zone = zone,
        }
    }

    pub fn set_is_hovered(&mut self, hovered: bool) {
        match self {
            Card::Spell(spell) => spell.hovered = hovered,
            Card::Site(site) => site.hovered = hovered,
            Card::Avatar(avatar) => avatar.hovered = hovered,
        }
    }

    pub fn set_is_selected(&mut self, selected: bool) {
        match self {
            Card::Spell(spell) => spell.selected = selected,
            Card::Site(site) => site.selected = selected,
            Card::Avatar(avatar) => avatar.selected = selected,
        }
    }
}

impl Clone for Card {
    fn clone(&self) -> Self {
        match self {
            Card::Spell(spell) => Card::Spell(spell.clone()),
            Card::Site(site) => Card::Site(site.clone()),
            Card::Avatar(avatar) => Card::Avatar(avatar.clone()),
        }
    }
}

#[derive(PartialEq)]
pub enum CardType {
    Spell,
    Site,
    Avatar,
}

impl CardType {
    pub fn get_dimensions(&self) -> Vec2 {
        match self {
            CardType::Site => Vec2::new(CARD_HEIGHT, CARD_WIDTH),
            _ => Vec2::new(CARD_WIDTH, CARD_HEIGHT),
        }
    }
}

pub fn get_image_path(name: &str) -> String {
    return format!("assets/images/cards/{}.webp", name);
}
