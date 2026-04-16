use crate::scene::game::GameData;
use egui::Rect;
use sorcerers::{card::Zone, game::PlayerId};

pub mod card_viewer;
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
    SelectionOverlay,
    CombatResolutionOverlay,
    ActionOverlay,
    CardViewer,
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
    DonePicking,
    OpenCardViewer {
        title: String,
        zone: Zone,
        controller_id: Option<PlayerId>,
    },
}

pub trait Component: std::fmt::Debug {
    fn update(&mut self, data: &mut GameData, ctx: &egui::Context) -> anyhow::Result<()>;

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
    ) -> anyhow::Result<Option<ComponentCommand>>;

    fn process_command(
        &mut self,
        _command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self);

    fn is_visible(&self) -> bool;

    fn get_component_type(&self) -> ComponentType;
}
