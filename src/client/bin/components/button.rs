use sorcerers::card::RenderableCard;

use crate::{components::Component, scene::game::Status};

#[derive(Debug)]
pub struct ButtonComponent {
    pub label: String,
}

#[async_trait::async_trait]
impl Component for ButtonComponent {
    async fn update(&mut self, cards: &[RenderableCard], status: Status) -> anyhow::Result<()> {
        Ok(())
    }

    async fn render(&mut self, status: &mut Status) {}

    fn toggle_visibility(&mut self) {}

    fn process_input(&mut self, in_turn: bool, status: &mut Status) {}
}
