use macroquad::math::{Rect, Vec2};

pub const CARD_OFFSET_X: f32 = 50.0;
pub const SCREEN_WIDTH: f32 = 1024.0;
pub const SCREEN_HEIGHT: f32 = 768.0;
pub const HAND_RECT: Rect = Rect::new(
    (SCREEN_WIDTH / 2.0) - 150.0,
    SCREEN_HEIGHT - 200.0,
    300.0,
    200.0,
);

pub const SCALE: f32 = 3.5;
pub const CARD_WIDTH: f32 = 384.0 / SCALE;
pub const CARD_HEIGHT: f32 = 537.0 / SCALE;

pub const SPELLBOOK_RECT: Rect = Rect::new(
    SCREEN_WIDTH - CARD_WIDTH - 20.0 - (CARD_HEIGHT - CARD_WIDTH) / 2.0,
    SCREEN_HEIGHT - 2.0 * CARD_HEIGHT - 40.0,
    CARD_WIDTH,
    CARD_HEIGHT,
);
pub const SPELLBOOK_SIZE: Vec2 = Vec2::new(CARD_WIDTH, CARD_HEIGHT);

pub const DISCARD_PILE_RECT: Rect = Rect::new(
    SCREEN_WIDTH - CARD_WIDTH - 20.0,
    SCREEN_HEIGHT - CARD_HEIGHT - 20.0,
    CARD_WIDTH,
    CARD_HEIGHT,
);
pub const DISCARD_PILE_SIZE: Vec2 = Vec2::new(CARD_WIDTH, CARD_HEIGHT);

pub const ATLASBOOK_RECT: Rect = Rect::new(
    SCREEN_WIDTH - CARD_HEIGHT - 20.0,
    SCREEN_HEIGHT - 2.0 * CARD_HEIGHT - 40.0 - CARD_WIDTH - 20.0,
    CARD_HEIGHT,
    CARD_WIDTH,
);
pub const ATLASBOOK_SIZE: Vec2 = Vec2::new(CARD_HEIGHT, CARD_WIDTH);
