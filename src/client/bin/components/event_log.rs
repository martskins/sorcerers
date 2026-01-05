use crate::{
    components::{Component, ComponentCommand, ComponentType},
    scene::game::{Game, GameData},
};
use macroquad::{
    math::{Rect, Vec2},
    ui::{self},
};

const EVENT_LOG_WINDOW: u64 = 10;

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

const FONT_SIZE: u16 = 16;

#[async_trait::async_trait]
impl Component for EventLogComponent {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        if self.visible {
            if let Some(last_event) = data.events.last() {
                if last_event.id != self.last_message_seen {
                    self.last_message_seen = last_event.id;
                }
                data.unseen_events = 0;
            }
        } else {
            if let Some(last_event) = data.events.last()
                && last_event.id != self.last_message_seen
            {
                let idx = data.events.iter().position(|e| e.id == self.last_message_seen);
                data.unseen_events = match idx {
                    Some(i) => data.events.len() - i - 1,
                    None => data.events.len(),
                };
            }
        }

        Ok(())
    }

    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }

        let visible = macroquad::ui::widgets::Window::new(
            EVENT_LOG_WINDOW,
            Vec2::new(self.rect.x, self.rect.y),
            Vec2::new(self.rect.w, self.rect.h),
        )
        .movable(true)
        .label("Event Log")
        .titlebar(true)
        .close_button(true)
        .ui(&mut ui::root_ui(), |ui| {
            for event in &data.events {
                let lines: Vec<String> = Game::wrap_text(event.formatted(), self.rect.w - 10.0, FONT_SIZE)
                    .lines()
                    .map(|line| line.to_string())
                    .collect();
                for line in lines {
                    ui.label(None, &line);
                }
            }
        });

        self.visible = visible;

        Ok(())
    }

    fn process_input(&mut self, _in_turn: bool, _data: &mut GameData) -> anyhow::Result<Option<ComponentCommand>> {
        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    async fn process_command(&mut self, command: &ComponentCommand) {
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
            } => self.rect = rect.clone(),
            _ => {}
        }
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::EventLog
    }
}
