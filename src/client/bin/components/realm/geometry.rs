use egui::{Pos2, Rect, Vec2, pos2, vec2};
use sorcerers::card::CardData;

use crate::config::CARD_ASPECT_RATIO;

fn cell_position(id: u8, mirror: bool) -> (u8, u8) {
    let idx = id - 1;
    let mut col = idx % 5;
    let mut row = 3 - (idx / 5);
    if mirror {
        col = 4 - col;
        row = 3 - row;
    }
    (row, col)
}

fn grid_vertex(realm_rect: &Rect, row: u8, col: u8) -> Pos2 {
    let depth = row as f32 / 4.0;
    let projected_depth = depth.powf(1.06);
    let y = realm_rect.min.y + realm_rect.height() * (0.10 + projected_depth * 0.78);
    let width = realm_rect.width() * (0.74 + projected_depth * 0.24);
    let left = realm_rect.center().x - width / 2.0;

    pos2(left + width * (col as f32 / 5.0), y)
}

pub(super) fn board_corners(realm_rect: &Rect) -> [Pos2; 4] {
    [
        grid_vertex(realm_rect, 0, 0),
        grid_vertex(realm_rect, 0, 5),
        grid_vertex(realm_rect, 4, 5),
        grid_vertex(realm_rect, 4, 0),
    ]
}

pub(super) fn cell_corners(realm_rect: &Rect, id: u8, mirror: bool, shrink: f32) -> [Pos2; 4] {
    let (row, col) = cell_position(id, mirror);
    let mut corners = [
        grid_vertex(realm_rect, row, col),
        grid_vertex(realm_rect, row, col + 1),
        grid_vertex(realm_rect, row + 1, col + 1),
        grid_vertex(realm_rect, row + 1, col),
    ];

    if shrink > 0.0 {
        let center = pos2(
            corners.iter().map(|p| p.x).sum::<f32>() / 4.0,
            corners.iter().map(|p| p.y).sum::<f32>() / 4.0,
        );
        for corner in &mut corners {
            let offset = *corner - center;
            let length = offset.length();
            if length > shrink {
                *corner = center + offset * ((length - shrink) / length);
            }
        }
    }

    corners
}

pub(super) fn cell_rect(realm_rect: &Rect, id: u8, mirror: bool) -> Rect {
    let corners = cell_corners(realm_rect, id, mirror, 0.0);
    bounding_rect(&corners)
}

pub(super) fn cell_inner_rect(realm_rect: &Rect, id: u8, mirror: bool, shrink: f32) -> Rect {
    let corners = cell_corners(realm_rect, id, mirror, shrink);
    bounding_rect(&corners)
}

pub(super) fn projected_card_dimensions(
    realm_rect: &Rect,
    cell_id: u8,
    mirror: bool,
    is_site: bool,
) -> Vec2 {
    let cell = cell_inner_rect(realm_rect, cell_id, mirror, 18.0);
    let plane = cell_corners(realm_rect, cell_id, mirror, 18.0);
    let horizontal = ((plane[1] - plane[0]).length() + (plane[2] - plane[3]).length()) / 2.0;
    let vertical = ((plane[3] - plane[0]).length() + (plane[2] - plane[1]).length()) / 2.0;
    let target_aspect = if is_site {
        1.0 / CARD_ASPECT_RATIO
    } else {
        CARD_ASPECT_RATIO
    };
    let width_fraction = if is_site { 0.33 } else { 0.242 };
    let height_fraction =
        ((width_fraction * horizontal) / (target_aspect * vertical)).clamp(0.12, 0.72);

    vec2(cell.width() * width_fraction, cell.height() * height_fraction)
}

pub(super) fn project_rect_in_cell(
    realm_rect: &Rect,
    cell_id: u8,
    mirror: bool,
    rect: Rect,
    shrink: f32,
    rotation: f32,
) -> [Pos2; 4] {
    let cell = cell_inner_rect(realm_rect, cell_id, mirror, shrink);
    let plane = cell_corners(realm_rect, cell_id, mirror, shrink);
    let center = rect.center();
    let half = rect.size() * 0.5;
    let (sin, cos) = rotation.sin_cos();
    let local_corners = [
        vec2(-half.x, -half.y),
        vec2(half.x, -half.y),
        vec2(half.x, half.y),
        vec2(-half.x, half.y),
    ];

    local_corners.map(|offset| {
        let point = pos2(
            center.x + cos * offset.x - sin * offset.y,
            center.y + sin * offset.x + cos * offset.y,
        );
        let u = ((point.x - cell.min.x) / cell.width()).clamp(0.0, 1.0);
        let v = ((point.y - cell.min.y) / cell.height()).clamp(0.0, 1.0);
        let top = plane[0] + (plane[1] - plane[0]) * u;
        let bottom = plane[3] + (plane[2] - plane[3]) * u;
        top + (bottom - top) * v
    })
}

fn bounding_rect(corners: &[Pos2; 4]) -> Rect {
    let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_x = corners.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let max_y = corners.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
}

pub(super) fn intersection_rect(realm_rect: &Rect, locations: &[u8], mirror: bool) -> Option<Rect> {
    let base_rect = cell_rect(realm_rect, 1, mirror);
    let width = spell_dimensions(&base_rect).x;
    let height = spell_dimensions(&base_rect).y;
    let start_rect = if mirror {
        cell_rect(realm_rect, locations[locations.len() - 1], mirror)
    } else {
        cell_rect(realm_rect, locations[0], mirror)
    };
    Some(Rect::from_min_size(
        pos2(
            start_rect.max.x - width / 2.0,
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
