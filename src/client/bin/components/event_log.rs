use crate::{
    components::Component,
    scene::game::{Game, GameData, Status},
};
use macroquad::{
    math::Vec2,
    ui::{self},
    window::screen_width,
};

const EVENT_LOG_WINDOW: u64 = 10;

#[derive(Debug)]
pub struct EventLogComponent {
    visible: bool,
    last_message_seen: uuid::Uuid,
}

impl EventLogComponent {
    pub fn new() -> Self {
        Self {
            visible: true,
            last_message_seen: uuid::Uuid::nil(),
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

    async fn render(&mut self, data: &mut GameData) {
        if !self.visible {
            return;
        }

        let window_width: f32 = screen_width() * 0.8;
        let visible =
            macroquad::ui::widgets::Window::new(EVENT_LOG_WINDOW, Vec2::new(0.0, 0.0), Vec2::new(window_width, 100.0))
                .movable(true)
                .label("Event Log")
                .titlebar(true)
                .close_button(true)
                .ui(&mut ui::root_ui(), |ui| {
                    for event in &data.events {
                        let lines: Vec<String> = Game::wrap_text(event.formatted(), window_width - 10.0, FONT_SIZE)
                            .lines()
                            .map(|line| line.to_string())
                            .collect();
                        for line in lines {
                            ui.label(None, &line);
                        }
                    }
                });

        self.visible = visible;
    }

    fn process_input(&mut self, in_turn: bool, data: &mut GameData) {}

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}
