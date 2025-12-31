use crate::scene::game::Status;
use sorcerers::card::RenderableCard;

pub mod player_hand;
pub mod realm;

#[async_trait::async_trait]
pub trait Component: std::fmt::Debug {
    async fn update(&mut self, cards: &[RenderableCard], status: Status) -> anyhow::Result<()>;
    async fn render(&mut self, status: &mut Status);
    fn toggle_visibility(&mut self);
    fn process_input(&mut self, in_turn: bool, status: &mut Status);
}
