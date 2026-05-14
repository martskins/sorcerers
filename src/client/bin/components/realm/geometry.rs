use egui::{Rect, Vec2, pos2, vec2};
use sorcerers::card::CardData;

use crate::config::CARD_ASPECT_RATIO;

pub(super) fn cell_rect(realm_rect: &Rect, id: u8, mirror: bool) -> Rect {
    let idx = id - 1;
    let mut col = idx % 5;
    let mut row = 3 - (idx / 5);
    if mirror {
        col = 4 - col;
        row = 3 - row;
    }
    let cell_width = realm_rect.width() / 5.0;
    let cell_height = realm_rect.height() / 4.0;
    Rect::from_min_size(
        pos2(
            realm_rect.min.x + col as f32 * cell_width,
            realm_rect.min.y + row as f32 * cell_height,
        ),
        vec2(cell_width, cell_height),
    )
}

pub(super) fn intersection_rect(realm_rect: &Rect, locations: &[u8], mirror: bool) -> Option<Rect> {
    let base_rect = cell_rect(realm_rect, 1, mirror);
    let width = spell_dimensions(&base_rect).x;
    let height = spell_dimensions(&base_rect).y;
    let cell_width = realm_rect.width() / 5.0;
    let start_rect = if mirror {
        cell_rect(realm_rect, locations[locations.len() - 1], mirror)
    } else {
        cell_rect(realm_rect, locations[0], mirror)
    };
    Some(Rect::from_min_size(
        pos2(
            start_rect.min.x + cell_width - width / 2.0,
            start_rect.min.y - height / 2.0,
        ),
        vec2(width, height),
    ))
}

fn card_width(cell_rect: &Rect) -> f32 {
    cell_rect.width() / 3.5
}

fn card_height(cell_rect: &Rect) -> f32 {
    card_width(cell_rect) / CARD_ASPECT_RATIO
}

pub(super) fn spell_dimensions(cell_rect: &Rect) -> Vec2 {
    vec2(card_width(cell_rect), card_height(cell_rect))
}

pub fn site_dimensions(cell_rect: &Rect) -> Vec2 {
    vec2(card_height(cell_rect), card_width(cell_rect))
}

pub(super) fn card_rotation(card: &CardData) -> f32 {
    if card.tapped {
        std::f32::consts::FRAC_PI_2
    } else {
        0.0
    }
}
