use macroquad::prelude::*;
use std::sync::{LazyLock, Mutex};

static MOUSE_STATUS: LazyLock<Mutex<Mouse>> = LazyLock::new(|| Mutex::new(Mouse::new()));

#[derive(Debug)]
pub struct Mouse {
    last_left_button_press: Option<chrono::DateTime<chrono::Utc>>,
    last_click_position: Option<Vec2>,
    enabled: bool,
}

impl Mouse {
    pub fn new() -> Self {
        Self {
            last_left_button_press: None,
            last_click_position: None,
            enabled: true,
        }
    }

    pub fn record_release() {
        let mut mouse_status = MOUSE_STATUS.lock().unwrap();
        mouse_status.last_left_button_press = None;
    }

    pub fn record_press() {
        let mut mouse_status = MOUSE_STATUS.lock().unwrap();
        mouse_status.last_left_button_press = Some(chrono::Utc::now());
        mouse_status.last_click_position = Some(mouse_position().into());
    }

    pub fn clicked() -> bool {
        !Mouse::dragging() && is_mouse_button_released(MouseButton::Left)
    }

    pub fn dragging() -> bool {
        let mouse_status = MOUSE_STATUS.lock().unwrap();
        if mouse_status.last_left_button_press.is_none() {
            return false;
        }

        let mouse_click_pos: Vec2 = mouse_status.last_click_position.unwrap();
        let current_mouse_pos: Vec2 = mouse_position().into();
        if mouse_click_pos.distance_squared(current_mouse_pos) > 25.0 {
            return true;
        }

        false
    }

    pub fn set_enabled(enabled: bool) {
        let mut mouse_status = MOUSE_STATUS.lock().unwrap();
        mouse_status.enabled = enabled;
    }

    pub fn enabled() -> bool {
        MOUSE_STATUS.lock().unwrap().enabled
    }
}
