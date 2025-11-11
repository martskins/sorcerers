use crate::{game::Game, render::Renderable};

struct Playground {}

impl Playground {
    pub fn new() -> Self {
        Playground {}
    }

    pub fn play(&self) {
        println!("Playing in the playground!");
    }
}

// impl Renderable for Playground {
//     fn render(
//         &self,
//         game: &mut Game,
//         handle: raylib::prelude::RaylibDrawHandle,
//     ) -> anyhow::Result<()> {
//         let image = raylib::prelude::Image::load_image("./assets/images/Realm.jpg")?;
//         let texture = game.handle.load_texture_from_image(&game.thread, &image)?;
//
//         handle.draw_texture_ex(&texture, Vector2::new(0.0, 0.0), 0.0, 1.2, Color::WHITE);
//         Ok(())
//     }
//
//     fn priority(&self) -> u8 {
//         return 0;
//     }
// }
