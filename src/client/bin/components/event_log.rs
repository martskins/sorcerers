use std::sync::Arc;

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    scene::game::GameData,
};
use egui::{Context, Painter, Rect, Ui, WidgetText, text::LayoutJob};

#[derive(Debug)]
pub struct EventLogComponent {
    visible: bool,
    last_message_seen: uuid::Uuid,
    rect: Rect,
}

impl EventLogComponent {
    pub fn new(rect: Rect) -> Self {
        Self {
            visible: false,
            last_message_seen: uuid::Uuid::nil(),
            rect,
        }
    }
}

impl Component for EventLogComponent {
    fn update(&mut self, data: &mut GameData, _ctx: &Context) -> anyhow::Result<()> {
        if self.visible {
            if let Some(last_event) = data.events.last() {
                if last_event.id != self.last_message_seen {
                    self.last_message_seen = last_event.id;
                }
                data.unseen_events = 0;
            }
        } else if let Some(last_event) = data.events.last()
            && last_event.id != self.last_message_seen
        {
            let idx = data
                .events
                .iter()
                .position(|e| e.id == self.last_message_seen);
            data.unseen_events = match idx {
                Some(i) => data.events.len() - i - 1,
                None => data.events.len(),
            };
        }
        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        _painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if !self.visible {
            return Ok(None);
        }
        let mut open = self.visible;
        egui::Window::new("Event Log")
            .open(&mut open)
            .movable(true)
            .resizable(true)
            .default_pos(self.rect.min)
            .default_size(self.rect.size())
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for event in &data.events {
                            let mut layout_job = LayoutJob::default();
                            layout_job.append(&event.formatted(), 0.0, egui::TextFormat::default());
                            ui.label(WidgetText::LayoutJob(Arc::new(layout_job)));
                        }
                    });
            });
        self.visible = open;
        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        match command {
            ComponentCommand::SetVisibility {
                component_type: ComponentType::EventLog,
                visible,
            } => {
                self.visible = *visible;
            }
            ComponentCommand::SetRect {
                component_type: ComponentType::EventLog,
                rect,
            } => {
                self.rect = *rect;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::EventLog
    }
}
