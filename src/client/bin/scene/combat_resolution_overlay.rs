use std::collections::HashMap;

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width},
    input::Mouse,
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use macroquad::{
    color::{BLACK, Color, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{Rect, RectOffset, Vec2},
    shapes::draw_rectangle,
    ui,
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{self, message::ClientMessage},
};

const FONT_SIZE: f32 = 24.0;

#[derive(Debug)]
pub struct CombatResolutionOverlay {
    card_rects: Vec<CardRect>,
    prompt: String,
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    defender_damage: HashMap<uuid::Uuid, u16>,
    damage: u16,
    shake_button_until: Option<chrono::DateTime<chrono::Utc>>,
    visible: bool,
}

impl CombatResolutionOverlay {
    pub async fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        attacker: CardData,
        defenders: Vec<CardData>,
        damage: u16,
    ) -> anyhow::Result<Self> {
        let mut textures = HashMap::with_capacity(defenders.len() + 1);
        let texture = TextureCache::get_card_texture(&attacker).await?;
        textures.insert(attacker.id, texture);
        for defender in &defenders {
            let texture = TextureCache::get_card_texture(&defender).await?;
            textures.insert(defender.id, texture);
        }

        let card_width = card_width()? * 1.2;
        let card_height = card_height()? * 1.2;
        let card_spacing = 20.0;

        // Attacker position (centered top half)
        let attacker_x = (screen_width() - card_width) / 2.0;
        let attacker_y = (screen_height() / 3.0) - (card_height / 2.0);

        let mut rects = Vec::new();
        rects.push(CardRect {
            rect: Rect::new(attacker_x, attacker_y, card_width, card_height),
            card: attacker.clone(),
            is_hovered: false,
            image: textures.get(&attacker.id).unwrap().clone(),
            is_selected: false,
        });

        // Defenders positions (centered bottom half)
        let defender_count = defenders.len();
        let defenders_area_width = defender_count as f32 * card_width + ((defender_count as f32 - 1.0) * card_spacing);
        let defenders_start_x = (screen_width() - defenders_area_width) / 2.0;
        let defenders_y = (3.0 * screen_height() / 5.0) - (card_height / 2.0);

        for (idx, defender) in defenders.iter().enumerate() {
            let x = defenders_start_x + idx as f32 * (card_width + card_spacing);
            rects.push(CardRect {
                rect: Rect::new(x, defenders_y, card_width, card_height),
                card: defender.clone(),
                image: textures.get(&defender.id).unwrap().clone(),
                is_hovered: false,
                is_selected: false,
            });
        }

        Ok(Self {
            client,
            game_id: game_id.clone(),
            card_rects: rects,
            prompt: format!("Distribute {} damage among defenders", damage),
            player_id: player_id.clone(),
            defender_damage: HashMap::new(),
            damage,
            shake_button_until: None,
            visible: true,
        })
    }
}

#[async_trait::async_trait]
impl Component for CombatResolutionOverlay {
    async fn update(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        if is_mouse_button_released(MouseButton::Left) {
            Mouse::set_enabled(true)?;
        }

        Ok(())
    }

    async fn process_command(&mut self, _command: &ComponentCommand, _data: &mut GameData) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::CombatResolutionOverlay
    }

    async fn process_input(&mut self, _in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentCommand>> {
        let mut skin = ui::root_ui().default_skin();
        skin.label_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(WHITE)
            .build();
        skin.button_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(BLACK)
            .build();
        ui::root_ui().push_skin(&skin);

        let damage_assigned = self.defender_damage.values().sum::<u16>();
        for card_rect in &self.card_rects {
            // Only show damage controls for defenders (skip attacker, which is first)
            if card_rect.card.controller_id != self.player_id {
                let card_id = card_rect.card.id;
                let damage = self.defender_damage.get(&card_id).copied().unwrap_or(0);

                let label_x = card_rect.rect.x + card_rect.rect.w / 2.0;
                let label_y = card_rect.rect.y + card_rect.rect.h + 10.0;

                if ui::root_ui().button(Vec2::new(label_x - 40.0, label_y), "-") {
                    if damage > 0 {
                        self.defender_damage.insert(card_id, damage - 1);
                    }
                }

                ui::root_ui().label(Vec2::new(label_x, label_y), damage.to_string().as_str());

                if ui::root_ui().button(Vec2::new(label_x + 40.0, label_y), "+") {
                    if damage_assigned < self.damage {
                        self.defender_damage.insert(card_id, damage + 1);
                    }
                }
            }
        }

        let defender_row_y = self
            .card_rects
            .iter()
            .skip(1)
            .map(|r| r.rect.y + r.rect.h)
            .fold(0.0, f32::max);
        let button_width = 150.0;
        let button_height = 40.0;
        let mut button_x = (screen_width() - button_width) / 2.0;
        let button_y = defender_row_y + 40.0;

        if let Some(shake_until) = self.shake_button_until {
            let now = chrono::Utc::now();
            if now < shake_until {
                let elapsed = (shake_until - now).num_milliseconds() as f32;
                let shake_magnitude = 3.0 * (elapsed / 300.0);
                let shake_x = (macroquad::rand::gen_range(-1.0, 1.0)) * shake_magnitude;
                button_x += shake_x;
            } else {
                self.shake_button_until = None;
            }
        }

        let pressed = ui::widgets::Button::new("Confirm")
            .position(Vec2::new(button_x, button_y))
            .size(Vec2::new(button_width, button_height))
            .ui(&mut ui::root_ui());
        if pressed {
            if damage_assigned != self.damage {
                self.shake_button_until = Some(chrono::Utc::now() + chrono::Duration::milliseconds(300));
            } else {
                self.client.send(ClientMessage::ResolveCombat {
                    game_id: self.game_id.clone(),
                    player_id: self.player_id.clone(),
                    damage_assignment: self.defender_damage.clone(),
                })?;
                self.visible = false;
            }
        }

        ui::root_ui().pop_skin();
        Ok(None)
    }

    async fn render(&mut self, _data: &mut GameData) -> anyhow::Result<()> {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.8),
        );

        let window_style = ui::root_ui()
            .style_builder()
            .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
            .build();
        let label_style = ui::root_ui()
            .style_builder()
            .font_size(FONT_SIZE as u16)
            .text_color(WHITE)
            .build();
        let skin = ui::Skin {
            window_style,
            label_style,
            ..ui::root_ui().default_skin()
        };

        ui::root_ui().push_skin(&skin);

        let mut skin = ui::root_ui().default_skin();
        skin.button_style = ui::root_ui()
            .style_builder()
            .color(Color::new(0.0, 0.0, 0.0, 0.0))
            .build();
        skin.label_style = ui::root_ui()
            .style_builder()
            .text_color(WHITE)
            .font_size(FONT_SIZE as u16)
            .build();
        ui::root_ui().push_skin(&skin);

        let text_dims = macroquad::text::measure_text(&self.prompt, None, FONT_SIZE as u16, 1.0);
        let wrapped_text = render::wrap_text(&self.prompt, screen_width() - 20.0, FONT_SIZE as u16);
        render::multiline_label(
            &wrapped_text,
            Vec2::new((screen_width() / 2.0) - (text_dims.width / 2.0), 30.0),
            FONT_SIZE as u16,
            &mut ui::root_ui(),
        );

        for card_rect in &self.card_rects {
            render::draw_card(card_rect, card_rect.card.controller_id == self.player_id, false);
        }

        ui::root_ui().pop_skin();
        Ok(())
    }
}
