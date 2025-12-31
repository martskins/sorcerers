use std::sync::Arc;

use crate::scene::game::Status;
use sorcerers::card::RenderableCard;

pub mod player_hand;
pub mod realm;

#[async_trait::async_trait]
pub trait Component: std::fmt::Debug {
    async fn update(&mut self, cards: &[RenderableCard], status: Status) -> anyhow::Result<()>;
    async fn render(&mut self);
}
