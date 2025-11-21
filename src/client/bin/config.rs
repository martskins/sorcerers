use macroquad::math::{Rect, Vec2};

pub const CARD_OFFSET_X: f32 = 50.0;
pub const SCREEN_WIDTH: f32 = 1024.0;
pub const SCREEN_HEIGHT: f32 = 768.0;

pub const HAND_RECT: Rect = Rect::new((SCREEN_WIDTH / 2.0) - 150.0, SCREEN_HEIGHT - 200.0, 300.0, 200.0);

pub const SCALE: f32 = 3.5;
pub const CARD_WIDTH: f32 = 384.0 / SCALE;
pub const CARD_HEIGHT: f32 = 537.0 / SCALE;
pub const SPELL_DIMENSIONS: Vec2 = Vec2::new(CARD_WIDTH, CARD_HEIGHT);
pub const SITE_DIMENSIONS: Vec2 = Vec2::new(CARD_HEIGHT, CARD_WIDTH);

pub const CARD_IN_PLAY_SCALE: f32 = 0.6;
pub const SPELLBOOK_RECT: Rect = Rect::new(
    REALM_RECT.x + REALM_RECT.w + 10.0,
    REALM_RECT.y + REALM_RECT.h - 2.0 * CARD_HEIGHT - 30.0,
    CARD_WIDTH,
    CARD_HEIGHT,
);

pub const DISCARD_PILE_RECT: Rect = Rect::new(
    REALM_RECT.x + REALM_RECT.w + 10.0,
    REALM_RECT.y + REALM_RECT.h - CARD_HEIGHT - 10.0,
    CARD_WIDTH,
    CARD_HEIGHT,
);

pub const ATLASBOOK_RECT: Rect = Rect::new(
    REALM_RECT.x + REALM_RECT.w + 10.0,
    REALM_RECT.y + REALM_RECT.h - 3.0 * CARD_HEIGHT - 10.0,
    CARD_HEIGHT,
    CARD_WIDTH,
);
pub const REALM_BACKGROUND_IMAGE: &str = "assets/images/Realm.jpg";
pub const SPELLBOOK_IMAGE: &str = "assets/images/cards/Spell Back.webp";
pub const ATLASBOOK_IMAGE: &str = "assets/images/cards/Site Back.webp";

pub const REALM_RECT: Rect = Rect::new(100.0, 0.0, SCREEN_WIDTH - 200.0, SCREEN_HEIGHT - HAND_RECT.h);
