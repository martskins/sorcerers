use macroquad::{
    color::{BLUE, Color, RED, WHITE},
    math::Vec2,
    shapes::{draw_line, draw_triangle_lines},
    text::draw_text,
    texture::{DrawTextureParams, draw_texture_ex},
};
use sorcerers::{
    card::Zone,
    game::{Element, Resources},
};

use crate::{components::Component, scene::game::GameData, texture_cache::TextureCache};

#[derive(Debug)]
pub struct PlayerStatusComponent {
    pub position: Vec2,
    pub visible: bool,
    pub player_id: uuid::Uuid,
}

impl PlayerStatusComponent {
    pub fn new(position: Vec2, player_id: uuid::Uuid) -> Self {
        Self {
            position,
            visible: true,
            player_id,
        }
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
        draw_text(player_name, self.position.x, self.position.y, FONT_SIZE, WHITE);

        const ICON_SIZE: f32 = 20.0;
        const NAME_BOTTOM_MARGIN: f32 = 7.0;
        let icon_y = self.position.y + NAME_BOTTOM_MARGIN;
        let health_text_y: f32 = self.position.y + NAME_BOTTOM_MARGIN + 20.0;
        let heart_texture = TextureCache::get_texture("assets/icons/heart.png").await;
        draw_texture_ex(
            &heart_texture,
            self.position.x,
            icon_y + 5.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE - 5.0, ICON_SIZE - 5.0)),
                ..Default::default()
            },
        );
        let health = format!("{}", resources.health);
        draw_text(&health, self.position.x + 22.0, health_text_y, FONT_SIZE, WHITE);

        let cards_texture = TextureCache::get_texture("assets/icons/cards.png").await;
        draw_texture_ex(
            &cards_texture,
            self.position.x + 52.0,
            icon_y + 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let cards_in_hand = format!(
            "{}",
            data.cards
                .iter()
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.zone == Zone::Hand)
                .count()
        );
        draw_text(&cards_in_hand, self.position.x + 77.0, health_text_y, FONT_SIZE, WHITE);

        let potion_texture = TextureCache::get_texture("assets/icons/potion.png").await;
        draw_texture_ex(
            &potion_texture,
            self.position.x + 95.0,
            icon_y + 4.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let mana_text = format!("{}", resources.mana);
        draw_text(&mana_text, self.position.x + 120.0, health_text_y, FONT_SIZE, WHITE);

        let tombstone_texture = TextureCache::get_texture("assets/icons/tombstone.png").await;
        draw_texture_ex(
            &tombstone_texture,
            self.position.x + 140.0,
            icon_y + 5.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let cards_in_cemetery = format!(
            "{}",
            data.cards
                .iter()
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.zone == Zone::Cemetery)
                .count()
        );
        draw_text(
            &cards_in_cemetery,
            self.position.x + 165.0,
            health_text_y,
            FONT_SIZE,
            WHITE,
        );

        if data.player_id == self.player_id {
            let message_texture = TextureCache::get_texture("assets/icons/message.png").await;
            draw_texture_ex(
                &message_texture,
                self.position.x + 140.0,
                icon_y - 20.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                    ..Default::default()
                },
            );
            let unseen_messages = format!("{}", data.unseen_events);
            draw_text(&unseen_messages, self.position.x + 165.0, icon_y, FONT_SIZE, WHITE);
        }

        let thresholds_y: f32 = self.position.y + 10.0 + 20.0 + 20.0;
        let fire_x = self.position.x;
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

    fn process_input(&mut self, _in_turn: bool, _data: &mut GameData) {}
}
