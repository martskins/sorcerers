use egui::{Context, Pos2};

pub struct Mouse;

impl Mouse {
    pub fn clicked(ctx: &Context) -> bool {
        ctx.input(|i| i.pointer.primary_released() && !i.pointer.is_decidedly_dragging())
    }

    pub fn dragging(ctx: &Context) -> bool {
        ctx.input(|i| i.pointer.is_decidedly_dragging())
    }

    pub fn position(ctx: &Context) -> Option<Pos2> {
        ctx.input(|i| i.pointer.latest_pos())
    }

    pub fn set_enabled(_enabled: bool) {}

    pub fn enabled() -> bool {
        true
    }
}
