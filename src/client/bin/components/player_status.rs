use macroquad::{
    color::{BLUE, Color, RED, WHITE},
    input::MouseButton,
    math::{Rect, Vec2},
    shapes::{draw_line, draw_triangle_lines},
    text::draw_text,
    texture::{DrawTextureParams, Texture2D, draw_texture_ex},
};
use sorcerers::{
    card::Zone,
    game::{Element, Resources},
};

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    scene::game::GameData,
    texture_cache::TextureCache,
};

#[derive(Debug)]
pub struct PlayerStatusComponent {
    visible: bool,
    player_id: uuid::Uuid,
    player: bool,
    rect: Rect,
}

enum Icon {
    Heart,
    Cards,
    Potion,
    Tombstone,
    Message,
}

impl PlayerStatusComponent {
    pub fn new(rect: Rect, player_id: uuid::Uuid, player: bool) -> Self {
        Self {
            visible: true,
            player_id,
            rect,
            player,
        }
    }

    fn icon_rect(&self, icon: &Icon) -> Rect {
        match icon {
            &Icon::Heart => Rect::new(self.rect.x, self.rect.y + 7.0, 20.0, 20.0),
            &Icon::Cards => Rect::new(self.rect.x + 52.0, self.rect.y + 4.0, 20.0, 20.0),
            &Icon::Potion => Rect::new(self.rect.x + 95.0, self.rect.y + 6.0, 20.0, 20.0),
            &Icon::Tombstone => Rect::new(self.rect.x + 140.0, self.rect.y + 7.0, 20.0, 20.0),
            &Icon::Message => Rect::new(self.rect.x + 140.0, self.rect.y - 13.0, 20.0, 20.0),
        }
    }

    async fn icon_texture(icon: &Icon) -> anyhow::Result<Texture2D> {
        match icon {
            &Icon::Heart => TextureCache::get_texture("assets/icons/heart.png").await,
            &Icon::Cards => TextureCache::get_texture("assets/icons/cards.png").await,
            &Icon::Potion => TextureCache::get_texture("assets/icons/potion.png").await,
            &Icon::Tombstone => TextureCache::get_texture("assets/icons/tombstone.png").await,
            &Icon::Message => TextureCache::get_texture("assets/icons/message.png").await,
        }
    }

    async fn draw_icon(&self, icon: &Icon, text: &str) -> anyhow::Result<()> {
        let texture = Self::icon_texture(icon).await?;
        let rect = self.icon_rect(icon);
        draw_texture_ex(
            &texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(rect.w, rect.h)),
                ..Default::default()
            },
        );
        draw_text(text, rect.x + rect.w + 5.0, rect.y + rect.h - 5.0, FONT_SIZE, WHITE);
        Ok(())
    }
}

const FONT_SIZE: f32 = 24.0;
const THRESHOLD_SYMBOL_SPACING: f32 = 18.0;
const SYMBOL_SIZE: f32 = 20.0;

fn render_threshold(x: f32, y: f32, value: u8, element: Element) {
    let text_offset_y = SYMBOL_SIZE * 0.8;
    draw_text(&value.to_string(), x, y + text_offset_y, FONT_SIZE, WHITE);

    const PURPLE: Color = Color::new(0.6, 0.2, 0.8, 1.0);
    const BROWN: Color = Color::new(0.6, 0.4, 0.2, 1.0);
    let element_color = match element {
        Element::Fire => RED,
        Element::Air => PURPLE,
        Element::Earth => BROWN,
        Element::Water => BLUE,
    };

    if element == Element::Earth || element == Element::Water {
        let v1 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING, y);
        let v2 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING + SYMBOL_SIZE / 2.0, y + SYMBOL_SIZE);
        let v3 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING + SYMBOL_SIZE, y);
        draw_triangle_lines(v1, v2, v3, 3.0, element_color);
    } else {
        let v1 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING, y + SYMBOL_SIZE);
        let v2 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING + SYMBOL_SIZE / 2.0, y);
        let v3 = Vec2::new(x + THRESHOLD_SYMBOL_SPACING + SYMBOL_SIZE, y + SYMBOL_SIZE);
        draw_triangle_lines(v1, v2, v3, 3.0, element_color);
    }

    if element == Element::Air || element == Element::Earth {
        let line_offset_y: f32 = SYMBOL_SIZE / 2.0;
        draw_line(
            x + THRESHOLD_SYMBOL_SPACING,
            y + line_offset_y,
            x + THRESHOLD_SYMBOL_SPACING + SYMBOL_SIZE,
            y + line_offset_y,
            2.0,
            element_color,
        );
    }
}

#[async_trait::async_trait]
impl Component for PlayerStatusComponent {
    async fn update(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        Ok(())
    }

    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        let resources = data.resources.get(&self.player_id).cloned().unwrap_or(Resources::new());
        let player_name = if data.player_id == self.player_id {
            "You"
        } else {
            "Them"
        };
        draw_text(player_name, self.rect.x, self.rect.y, FONT_SIZE, WHITE);

        let health = format!("{}", data.avatar_health.get(&self.player_id).cloned().unwrap_or(0));
        Self::draw_icon(self, &Icon::Heart, &health).await?;

        let cards_in_hand = format!(
            "{}",
            data.cards
                .iter()
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.zone == Zone::Hand)
                .count()
        );
        Self::draw_icon(self, &Icon::Cards, &cards_in_hand).await?;

        let mana_text = format!("{}", resources.mana);
        Self::draw_icon(self, &Icon::Potion, &mana_text).await?;

        let cards_in_cemetery = format!(
            "{}",
            data.cards
                .iter()
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.zone == Zone::Cemetery)
                .count()
        );
        Self::draw_icon(self, &Icon::Tombstone, &cards_in_cemetery).await?;

        if data.player_id == self.player_id {
            let unseen_messages = format!("{}", data.unseen_events);
            Self::draw_icon(self, &Icon::Message, &unseen_messages).await?;
        }

        let thresholds_y: f32 = self.rect.y + 10.0 + 20.0 + 20.0;
        let fire_x = self.rect.x;
        let fire_y = thresholds_y;
        render_threshold(fire_x, fire_y, resources.thresholds.fire, Element::Fire);

        let air_x = fire_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let air_y = thresholds_y;
        render_threshold(air_x, air_y, resources.thresholds.air, Element::Air);

        let earth_x = air_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let earth_y = thresholds_y;
        render_threshold(earth_x, earth_y, resources.thresholds.earth, Element::Earth);

        let water_x = earth_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let water_y = thresholds_y;
        render_threshold(water_x, water_y, resources.thresholds.water, Element::Water);

        Ok(())
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    async fn process_input(
        &mut self,
        _in_turn: bool,
        _data: &mut GameData,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let mouse_position = macroquad::input::mouse_position();
        let clicked = macroquad::input::is_mouse_button_released(MouseButton::Left);
        if clicked && self.icon_rect(&Icon::Message).contains(mouse_position.into()) {
            return Ok(Some(ComponentCommand::SetVisibility {
                component_type: ComponentType::EventLog,
                visible: true,
            }));
        }

        Ok(None)
    }

    async fn process_command(&mut self, command: &ComponentCommand) -> anyhow::Result<()> {
        match command {
            ComponentCommand::SetRect {
                component_type: ComponentType::PlayerStatus,
                rect,
            } => {
                self.rect = rect.clone();
                if self.player {
                    self.rect.y = crate::config::screen_rect()?.h - 90.0;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::PlayerStatus
    }
}
