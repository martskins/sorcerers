pub trait Renderable {
    // fn render(&self, draw_handle: &mut raylib::prelude::RaylibDrawHandle) -> anyhow::Result<()>;
    fn priority(&self) -> u8;
}
