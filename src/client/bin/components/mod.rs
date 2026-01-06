use macroquad::math::Rect;

use crate::scene::game::GameData;

pub mod event_log;
pub mod player_hand;
pub mod player_status;
pub mod realm;

#[derive(Debug, Clone)]
pub enum ComponentType {
    EventLog,
    PlayerStatus,
    PlayerHand,
    Realm,
}

#[derive(Debug, Clone)]
pub enum ComponentCommand {
    SetVisibility {
        component_type: ComponentType,
        visible: bool,
    },
    SetRect {
        component_type: ComponentType,
        rect: Rect,
    },
}

#[async_trait::async_trait]
pub trait Component: std::fmt::Debug {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()>;
    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()>;
    async fn process_input(&mut self, in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentCommand>>;
    async fn process_command(&mut self, command: &ComponentCommand);
    fn toggle_visibility(&mut self);
    fn get_component_type(&self) -> ComponentType;
}
