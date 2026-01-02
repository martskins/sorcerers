use crate::scene::game::GameData;

pub mod event_log;
pub mod player_hand;
pub mod player_status;
pub mod realm;

#[derive(Debug, Clone)]
pub enum ComponentAction {
    OpenEventLog,
}

#[async_trait::async_trait]
pub trait Component: std::fmt::Debug {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()>;
    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()>;
    fn toggle_visibility(&mut self);
    fn process_input(&mut self, in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentAction>>;
    fn process_action(&mut self, _action: &ComponentAction) {}
}
