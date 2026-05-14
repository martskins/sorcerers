use egui::{Rect, Vec2, pos2, vec2};
use sorcerers::card::CardData;

use crate::config::CARD_ASPECT_RATIO;

pub const DEFAULT_THUMB_W: f32 = 80.0;
pub const CARD_PAD: f32 = 10.0;
pub const STACK_STRIP: f32 = 22.0;
pub const STACK_MAX_PER_COLUMN: usize = 10;
pub const SITE_ROWS_PER_COLUMN: usize = 4;
pub const SITE_SPACING_X: f32 = 20.0;

#[derive(Debug, Clone, Copy)]
pub struct CardDims {
    pub spell: Vec2,
    pub site: Vec2,
}

impl CardDims {
    pub fn from_spell_width(width: f32) -> Self {
        let height = width / CARD_ASPECT_RATIO;
        Self {
            spell: vec2(width, height),
            site: vec2(height, width),
        }
    }

    pub fn from_spell_height(height: f32) -> Self {
        let width = height * CARD_ASPECT_RATIO;
        Self {
            spell: vec2(width, height),
            site: vec2(height, width),
        }
    }

    pub fn for_card(&self, card: &CardData) -> Vec2 {
        if card.is_site() {
            self.site
        } else {
            self.spell
        }
    }
}

#[derive(Debug, Clone)]
pub struct HandLayout {
    pub spell_spacing: f32,
    pub spells_width: f32,
    pub site_spacing_y: f32,
    pub sites_height: f32,
    pub total_width: f32,
}

impl HandLayout {
    pub fn new(spell_count: usize, site_count: usize, dims: CardDims, available_width: f32) -> Self {
        let min_visible_width = dims.spell.x * 0.25;
        let spell_spacing = if spell_count > 1 {
            ((available_width - dims.spell.x) / (spell_count as f32 - 1.0))
                .min(dims.spell.x - min_visible_width)
                .max(10.0)
        } else {
            0.0
        };

        let spells_width = row_width(spell_count, dims.spell.x, spell_spacing);
        let site_spacing_y = (dims.site.y * 0.15).max(20.0);
        let site_columns = site_count.div_ceil(SITE_ROWS_PER_COLUMN);
        let sites_width = if site_count == 0 {
            0.0
        } else {
            site_columns as f32 * dims.site.x
                + site_columns.saturating_sub(1) as f32 * SITE_SPACING_X
        };
        let sites_height = if site_count == 0 {
            0.0
        } else {
            let rows = site_count.min(SITE_ROWS_PER_COLUMN);
            dims.site.y + rows.saturating_sub(1) as f32 * site_spacing_y
        };
        let total_width = spells_width
            + if site_count == 0 {
                0.0
            } else {
                SITE_SPACING_X + sites_width
            };

        Self {
            spell_spacing,
            spells_width,
            site_spacing_y,
            sites_height,
            total_width,
        }
    }
}

pub fn row_width(count: usize, item_width: f32, spacing: f32) -> f32 {
    if count == 0 {
        0.0
    } else {
        item_width + (count as f32 - 1.0) * spacing
    }
}

pub fn hand_content_size(layout: &HandLayout, dims: CardDims, available_width: f32) -> Vec2 {
    vec2(
        layout.total_width.max(available_width).max(dims.spell.x),
        dims.spell.y.max(layout.sites_height).max(dims.site.y),
    )
}

pub fn spell_rect(container: Rect, layout: &HandLayout, dims: CardDims, index: usize) -> Rect {
    let start_x = container.min.x + (container.width() - layout.total_width) / 2.0;
    let y = container.center().y - dims.spell.y / 2.0;
    Rect::from_min_size(pos2(start_x + index as f32 * layout.spell_spacing, y), dims.spell)
}

pub fn site_rect(container: Rect, layout: &HandLayout, dims: CardDims, index: usize) -> Rect {
    let start_x = container.min.x + (container.width() - layout.total_width) / 2.0;
    let sites_x = start_x + layout.spells_width + SITE_SPACING_X;
    let col = index / SITE_ROWS_PER_COLUMN;
    let row = index % SITE_ROWS_PER_COLUMN;
    let x = sites_x + col as f32 * (dims.site.x + SITE_SPACING_X);
    let y = container.center().y - dims.spell.y / 2.0 + row as f32 * layout.site_spacing_y;
    Rect::from_min_size(pos2(x, y), dims.site)
}
