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

fn board_rect(realm_rect: &Rect) -> Rect {
    Rect::from_min_max(
        pos2(realm_rect.min.x + 60.0, realm_rect.min.y + 72.0),
        pos2(realm_rect.max.x - 60.0, realm_rect.max.y - 96.0),
    )
}

fn board_cell_rect(realm_rect: &Rect, id: u8, mirror: bool, shrink: f32) -> Rect {
    let (row, col) = cell_position(id, mirror);
    let board = board_rect(realm_rect);
    let cell = vec2(board.width() / 5.0, board.height() / 4.0);
    Rect::from_min_max(
        pos2(
            board.min.x + cell.x * col as f32,
            board.min.y + cell.y * row as f32,
        ),
        pos2(
            board.min.x + cell.x * (col + 1) as f32,
            board.min.y + cell.y * (row + 1) as f32,
        ),
    )
    .shrink(shrink)
}

pub(super) fn board_corners(realm_rect: &Rect) -> [Pos2; 4] {
    let board = board_rect(realm_rect);
    [
        board.left_top(),
        board.right_top(),
        board.right_bottom(),
        board.left_bottom(),
    ]
}

pub(super) fn cell_corners(realm_rect: &Rect, id: u8, mirror: bool, shrink: f32) -> [Pos2; 4] {
    let rect = board_cell_rect(realm_rect, id, mirror, shrink);
    [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ]
}

pub(super) fn cell_rect(realm_rect: &Rect, id: u8, mirror: bool) -> Rect {
    board_cell_rect(realm_rect, id, mirror, 0.0)
}

pub(super) fn cell_inner_rect(realm_rect: &Rect, id: u8, mirror: bool, shrink: f32) -> Rect {
    board_cell_rect(realm_rect, id, mirror, shrink)
}

pub(super) fn projected_card_dimensions(
    realm_rect: &Rect,
    cell_id: u8,
    mirror: bool,
    is_site: bool,
) -> Vec2 {
    let cell = cell_inner_rect(realm_rect, cell_id, mirror, 18.0);
    let target_aspect = if is_site {
        1.0 / CARD_ASPECT_RATIO
    } else {
        CARD_ASPECT_RATIO
    };
    let width_fraction = if is_site { 0.33 } else { 0.242 };
    let width = cell.width() * width_fraction;
    vec2(width, width / target_aspect)
}

pub(super) fn card_corners(rect: Rect, rotation: f32) -> [Pos2; 4] {
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
        pos2(
            center.x + cos * offset.x - sin * offset.y,
            center.y + sin * offset.x + cos * offset.y,
        )
    })
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
