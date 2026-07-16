use egui::{Color32, Context, FontFamily, FontId};

pub const APP_BG: Color32 = Color32::from_rgb(8, 9, 13);
// Interface surfaces are soot-black and slightly translucent, so the table art
// stays present behind them. Blue is reserved for actions, not containers.
pub const PANEL_BG: Color32 = Color32::from_rgba_premultiplied(11, 15, 16, 238);
pub const PANEL_BORDER: Color32 = Color32::from_rgb(91, 94, 85);
pub const SURFACE_INSET: Color32 = Color32::from_rgb(11, 18, 20);
pub const SURFACE_INSET_FOCUSED: Color32 = Color32::from_rgb(17, 29, 33);
pub const INPUT_BORDER: Color32 = Color32::from_rgb(79, 91, 88);
pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(226, 228, 220);
pub const OVERLAY_SCRIM: Color32 = Color32::from_rgba_unmultiplied_const(0, 0, 0, 204);
pub const ACTION: Color32 = Color32::from_rgb(55, 112, 155);
pub const ACTION_HOVERED: Color32 = Color32::from_rgb(77, 145, 190);
pub const ACTION_ACTIVE: Color32 = Color32::from_rgb(35, 78, 118);
pub const SELECTION: Color32 = Color32::from_rgb(78, 150, 190);
pub const PICKABLE: Color32 = Color32::from_rgb(130, 226, 144);
pub const TURN_READY: Color32 = Color32::from_rgb(125, 226, 146);
pub const TURN_WAITING: Color32 = Color32::from_rgb(172, 176, 190);

// Elemental identity is used only where the game is describing an element,
// a threshold, or a deck's elemental character.
pub const ELEMENT_FIRE: Color32 = Color32::from_rgb(220, 70, 40);
pub const ELEMENT_AIR: Color32 = Color32::from_rgb(160, 90, 220);
pub const ELEMENT_EARTH: Color32 = Color32::from_rgb(140, 100, 40);
pub const ELEMENT_WATER: Color32 = Color32::from_rgb(50, 150, 230);

// The realm is a physical play surface, not another application panel.
pub const TABLE_RAIL: Color32 = Color32::from_rgb(10, 14, 19);
pub const TABLE_FELT: Color32 = Color32::from_rgba_unmultiplied_const(22, 33, 28, 238);
pub const TABLE_EDGE: Color32 = Color32::from_rgb(103, 86, 48);

pub const BUTTON_HEIGHT: f32 = 48.0;

/// Animation can be disabled for assistive technology or by launching with
/// `SORCERERS_REDUCE_MOTION=1`.
pub fn animations_enabled(ctx: &Context) -> bool {
    !ctx.memory(|memory| memory.options.screen_reader)
        && std::env::var_os("SORCERERS_REDUCE_MOTION").as_deref() != Some("1".as_ref())
}

pub fn animation_time(ctx: &Context, standard_time: f32) -> f32 {
    if animations_enabled(ctx) {
        standard_time
    } else {
        0.0
    }
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("SpectralDisplay".into()))
}

pub fn display_bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("SpectralDisplayBold".into()))
}
