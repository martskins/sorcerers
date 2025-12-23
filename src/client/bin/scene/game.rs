use crate::{
    config::*,
    render::{CardRect, CellRect},
    scene::Scene,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{BLUE, Color, DARKGRAY, DARKGREEN, GRAY, GREEN, RED, WHITE},
    input::{MouseButton, is_mouse_button_released, mouse_position},
    math::{Rect, RectOffset, Vec2},
    shapes::{
        DrawRectangleParams, draw_circle_lines, draw_line, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines,
        draw_rectangle_lines_ex, draw_triangle_lines,
    },
    text::draw_text,
    texture::{DrawTextureParams, draw_texture_ex},
    ui::{self, hash},
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::{CardType, Modifier, Plane, RenderableCard, Zone},
    game::{Element, PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, ServerMessage},
    },
};
use std::collections::HashMap;

const FONT_SIZE: f32 = 24.0;
const THRESHOLD_SYMBOL_SPACING: f32 = 18.0;
const SYMBOL_SIZE: f32 = 20.0;

fn draw_vortex_icon(x: f32, y: f32, size: f32, color: Color) {
    use macroquad::shapes::draw_line;
    let turns = 2.0;
    let segments = 24;
    let mut prev = (x + size / 2.0, y + size / 2.0);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = turns * std::f32::consts::TAU * t;
        let radius = (size / 2.0) * t;
        let px = x + size / 2.0 + radius * angle.cos();
        let py = y + size / 2.0 + radius * angle.sin();
        draw_line(prev.0, prev.1, px, py, 2.0, color);
        prev = (px, py);
    }
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Idle,
    SelectingAction { prompt: String },
    SelectingCard { cards: Vec<uuid::Uuid> },
    SelectingZone { zones: Vec<Zone> },
}

#[derive(Debug)]
pub struct Game {
    pub player_id: PlayerId,
    pub game_id: uuid::Uuid,
    pub card_rects: Vec<CardRect>,
    pub cell_rects: Vec<CellRect>,
    pub cards: Vec<RenderableCard>,
    pub resources: HashMap<PlayerId, Resources>,
    pub client: networking::client::Client,
    pub current_player: PlayerId,
    pub is_player_one: bool,
    actions: Vec<String>,
    status: Status,
}

impl Game {
    pub fn new(player_id: uuid::Uuid, client: networking::client::Client) -> Self {
        let cells = (0..20)
            .map(|i| {
                let rect = cell_rect(i + 1, false);
                CellRect { id: i as u8 + 1, rect }
            })
            .collect();
        Self {
            player_id: player_id,
            card_rects: Vec::new(),
            cards: Vec::new(),
            game_id: uuid::Uuid::nil(),
            cell_rects: cells,
            client,
            current_player: uuid::Uuid::nil(),
            is_player_one: false,
            resources: HashMap::new(),
            actions: Vec::new(),
            status: Status::Idle,
        }
    }

    fn is_players_turn(&self, player_id: &PlayerId) -> bool {
        self.current_player == *player_id
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        let mouse_position = mouse_position().into();
        self.resize_cells().await?;
        self.compute_hand_rects().await?;
        self.compute_realm_rects().await?;
        self.handle_click(mouse_position);
        Ok(())
    }

    async fn resize_cells(&mut self) -> anyhow::Result<()> {
        let mirror = !self.is_player_one;
        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(cell.id, mirror);
        }

        Ok(())
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        if self.game_id.is_nil() {
            let time = macroquad::time::get_time();
            let dot_count = ((time * 2.0) as usize % 3) + 1;
            let mut dots = ".".repeat(dot_count);
            dots += &" ".repeat(3 - dot_count);
            let message = format!("Looking for match{}", dots);

            let screen_rect = screen_rect();
            let text_dimensions = macroquad::text::measure_text(&message, None, FONT_SIZE as u16, 1.0);
            let x = screen_rect.w / 2.0 - text_dimensions.width / 2.0;
            let y = screen_rect.h / 2.0 - text_dimensions.height / 2.0;

            draw_text(&message, x, y, 32.0, WHITE);
            return Ok(());
        }

        self.render_background().await;
        self.render_grid().await;
        self.render_realm().await;
        // self.render_deck().await;
        // self.render_cemetery().await?;
        self.render_gui().await?;
        self.render_player_hand().await;
        self.render_card_preview().await?;
        Ok(())
    }

    pub async fn process_message(&mut self, message: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match message {
            ServerMessage::PickZone { zones, .. } => {
                self.status = Status::SelectingZone { zones: zones.clone() };
                Ok(None)
            }
            ServerMessage::PickCard { cards, .. } => {
                self.status = Status::SelectingCard { cards: cards.clone() };
                Ok(None)
            }
            ServerMessage::PickAction { prompt, actions, .. } => {
                self.actions = actions.clone();
                self.status = Status::SelectingAction {
                    prompt: prompt.to_string(),
                };
                Ok(None)
            }
            ServerMessage::GameStarted { game_id, player1, .. } => {
                // Flip the board for player 2. Use player1 instead of the is_player_one method
                // because state is not set at this point.
                self.is_player_one = player1 == &self.player_id;
                if !self.is_player_one {
                    for cell in &mut self.cell_rects {
                        let new_id: i8 = cell.id as i8 - 21;
                        cell.id = new_id.abs() as u8;
                    }
                }

                self.game_id = game_id.clone();
                Ok(None)
            }
            ServerMessage::Sync {
                cards,
                current_player,
                resources,
            } => {
                // Sort so that cards that are submerged or burrowed are drawn first, then sites, then
                // cards on the surface and then cards in the air.
                let mut cards = cards.clone();
                cards.sort_by(|a, b| match (&a.plane, &b.plane) {
                    (Plane::Air, Plane::Air)
                    | (Plane::Burrowed, Plane::Burrowed)
                    | (Plane::Submerged, Plane::Submerged) => std::cmp::Ordering::Equal,
                    (Plane::Surface, Plane::Surface) => match (&a.card_type, &b.card_type) {
                        (CardType::Site, _) => std::cmp::Ordering::Less,
                        (_, _) => std::cmp::Ordering::Equal,
                    },
                    (Plane::Air, _) => std::cmp::Ordering::Greater,
                    (Plane::Surface, Plane::Air) => std::cmp::Ordering::Less,
                    (Plane::Surface, _) => std::cmp::Ordering::Greater,
                    (Plane::Burrowed, Plane::Air) => std::cmp::Ordering::Less,
                    (Plane::Burrowed, Plane::Surface) => std::cmp::Ordering::Less,
                    (_, _) => std::cmp::Ordering::Equal,
                });

                self.cards = cards.clone();
                self.current_player = current_player.clone();
                self.resources = resources.clone();
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_click(&mut self, mouse_position: Vec2) {
        self.handle_card_click(mouse_position);
        self.handle_square_click(mouse_position);
    }

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

    fn render_player_card(x: f32, y: f32, resources: &Resources, player_name: &str) {
        draw_text(player_name, x, y, FONT_SIZE, WHITE);

        let health = format!("Health: {}", resources.health);
        draw_text(&health, x, y + 20.0, FONT_SIZE, WHITE);

        let mana_text = format!("Mana: {}", resources.mana);
        draw_text(&mana_text, x, y + 40.0, FONT_SIZE, WHITE);

        let thresholds_y: f32 = y + 10.0 + 20.0 + 20.0;
        let fire_x = x;
        let fire_y = thresholds_y;
        Game::render_threshold(fire_x, fire_y, resources.thresholds.fire, Element::Fire);

        let air_x = fire_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let air_y = thresholds_y;
        Game::render_threshold(air_x, air_y, resources.thresholds.air, Element::Air);

        let earth_x = air_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let earth_y = thresholds_y;
        Game::render_threshold(earth_x, earth_y, resources.thresholds.earth, Element::Earth);

        let water_x = earth_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let water_y = thresholds_y;
        Game::render_threshold(water_x, water_y, resources.thresholds.water, Element::Water);
    }

    async fn render_gui(&mut self) -> anyhow::Result<()> {
        let screen_rect = screen_rect();
        let base_x: f32 = 20.0;
        let player_y: f32 = screen_rect.h - 90.0;
        let resources = self.resources.get(&self.player_id).cloned().unwrap_or(Resources::new());
        let player1 = if self.is_player_one { "Player 1" } else { "Player 2" };
        let player2 = if self.is_player_one { "Player 2" } else { "Player 1" };
        Game::render_player_card(base_x, player_y, &resources, player1);

        const OPPONENT_Y: f32 = 25.0;
        let opponent_resources = self
            .resources
            .iter()
            .find(|(player_id, _)| **player_id != self.player_id)
            .map(|(_, resources)| resources.clone())
            .unwrap_or(Resources::new());
        Game::render_player_card(base_x, OPPONENT_Y, &opponent_resources, player2);

        let turn_label = if self.is_players_turn(&self.player_id) {
            "Your Turn"
        } else {
            "Opponent's Turn"
        };

        draw_text(turn_label, screen_rect.w / 2.0 - 50.0, 30.0, FONT_SIZE, WHITE);

        let is_in_turn = self.current_player == self.player_id;
        if is_in_turn {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 100.0, screen_rect.h - 40.0), "End Turn") {
                self.client.send(ClientMessage::EndTurn {
                    player_id: self.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        }

        match self.status {
            // Status::SelectingCard { ref cards } => {
            //     let valid_cards: Vec<&CardRect> = self.card_rects.iter().filter(|c| cards.contains(&c.id)).collect();
            //     for card in valid_cards {
            //         draw_rectangle_lines(card.rect.x, card.rect.y, card.rect.w, card.rect.h, 3.0, WHITE);
            //     }
            // }
            Status::SelectingAction { ref prompt } => {
                // Draw semi-transparent overlay behind the action selection window
                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.6),
                );

                let window_style = ui::root_ui()
                    .style_builder()
                    .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
                    .build();
                let skin = ui::Skin {
                    window_style,
                    ..ui::root_ui().default_skin()
                };

                ui::root_ui().push_skin(&skin);

                let prompt = prompt.clone();
                let button_height = 30.0;
                let window_size = Vec2::new(400.0, (button_height + 10.0) * self.actions.len() as f32 + 20.0 + 50.0);
                ui::root_ui().window(
                    hash!(),
                    Vec2::new(
                        screen_width() / 2.0 - window_size.x / 2.0,
                        screen_height() / 2.0 - window_size.y / 2.0,
                    ),
                    window_size,
                    |ui| {
                        ui::widgets::Label::new(&prompt)
                            .position(Vec2::new(5.0, 5.0))
                            .multiline(10.0)
                            .ui(ui);
                        for (idx, action) in self.actions.iter().enumerate() {
                            let button_pos =
                                Vec2::new(window_size.x * 0.1, (button_height + 10.0) * (idx as f32 + 1.0));
                            let clicked = ui::widgets::Button::new(action.as_str())
                                .position(button_pos)
                                .size(Vec2::new(window_size.x * 0.8, button_height))
                                .ui(ui);
                            if clicked {
                                self.client
                                    .send(ClientMessage::PickAction {
                                        game_id: self.game_id,
                                        player_id: self.player_id,
                                        action_idx: idx,
                                    })
                                    .unwrap();
                                self.status = Status::Idle;
                            }
                        }
                    },
                );

                ui::root_ui().pop_skin();
            }
            _ => {}
        }

        Ok(())
    }

    async fn render_card_preview(&self) -> anyhow::Result<()> {
        let selected_card = self.card_rects.iter().find(|card_display| card_display.is_hovered);
        let screen_rect = screen_rect();

        if let Some(card_display) = selected_card {
            const PREVIEW_SCALE: f32 = 2.7;
            let mut rect = card_display.rect;
            rect.w *= PREVIEW_SCALE;
            rect.h *= PREVIEW_SCALE;

            let preview_x = 20.0;
            let preview_y = screen_rect.h - rect.h - 20.0;
            draw_texture_ex(
                &card_display.image,
                preview_x,
                preview_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(rect.w, rect.h)),
                    ..Default::default()
                },
            );
        }

        Ok(())
    }

    async fn render_cemetery(&self) -> anyhow::Result<()> {
        let discarded_cards = self
            .card_rects
            .iter()
            .filter(|card_display| card_display.zone == Zone::Cemetery)
            .collect::<Vec<&CardRect>>();
        for card in discarded_cards {
            let cemetery_rect = cemetery_rect();
            draw_texture_ex(
                &card.image,
                cemetery_rect.x,
                cemetery_rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(cemetery_rect.size() * CARD_IN_PLAY_SCALE),
                    ..Default::default()
                },
            );
        }

        Ok(())
    }

    fn handle_card_click(&mut self, mouse_position: Vec2) {
        if let Status::SelectingAction { .. } = &self.status {
            return;
        }

        if self.current_player != self.player_id {
            return;
        }

        let mut hovered_card_index = None;
        for (idx, card_display) in self.card_rects.iter().enumerate() {
            if card_display.rect.contains(mouse_position.into()) {
                hovered_card_index = Some(idx);
            };
        }

        for card in &mut self.card_rects {
            card.is_hovered = false;
        }

        if let Some(idx) = hovered_card_index {
            self.card_rects.get_mut(idx).unwrap().is_hovered = true;
        }

        match &self.status {
            Status::Idle => {
                let mut card_selected = None;
                for card_display in &mut self
                    .card_rects
                    .iter_mut()
                    .filter(|c| matches!(c.zone, Zone::Realm(_)) || c.zone == Zone::Hand)
                {
                    if card_display.is_hovered && is_mouse_button_released(MouseButton::Left) {
                        card_selected = Some(card_display.id.clone());
                        break;
                    };
                }

                if card_selected.is_some() {
                    self.client
                        .send(ClientMessage::ClickCard {
                            card_id: card_selected.unwrap(),
                            player_id: self.player_id,
                            game_id: self.game_id,
                        })
                        .unwrap();
                }
            }
            Status::SelectingCard { cards, .. } => {
                let valid_cards: Vec<&CardRect> = self.card_rects.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
                    let card = self.card_rects.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        self.status = Status::Idle;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_square_click(&mut self, mouse_position: Vec2) {
        if let Status::SelectingAction { .. } = &self.status {
            return;
        }

        match &self.status {
            Status::SelectingZone { zones } => {
                let zones = zones.clone();
                for (idx, cell) in self.cell_rects.iter().enumerate() {
                    if zones.iter().find(|i| i == &&Zone::Realm(cell.id)).is_none() {
                        continue;
                    }

                    if cell.rect.contains(mouse_position.into()) {
                        let square = self.cell_rects[idx].id;
                        if is_mouse_button_released(MouseButton::Left) {
                            self.client
                                .send(ClientMessage::PickSquare {
                                    player_id: self.player_id.clone(),
                                    game_id: self.game_id.clone(),
                                    square,
                                })
                                .unwrap();

                            self.status = Status::Idle;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn render_grid(&self) {
        let grid_color = WHITE;
        let grid_thickness = 1.0;
        for cell_display in &self.cell_rects {
            draw_rectangle_lines(
                cell_display.rect.x,
                cell_display.rect.y,
                cell_display.rect.w,
                cell_display.rect.h,
                grid_thickness,
                grid_color,
            );

            draw_text(
                &cell_display.id.to_string(),
                cell_display.rect.x + 5.0,
                cell_display.rect.y + 15.0,
                12.0,
                GRAY,
            );

            // Draw a UI button at the top right corner as a placeholder for an icon
            let button_size = 18.0;
            let button_x = cell_display.rect.x + cell_display.rect.w - button_size - 4.0;
            let button_y = cell_display.rect.y + 4.0;
            let button_pos = Vec2::new(button_x, button_y);
            let button_dim = Vec2::new(button_size, button_size);
            let button = ui::widgets::Button::new("+")
                .position(button_pos)
                .size(button_dim)
                .ui(&mut ui::root_ui());

            if button {
                println!("Clicked cell {}", cell_display.id);
            }

            if let Status::SelectingZone { zones } = &self.status {
                if zones.iter().find(|i| i == &&Zone::Realm(cell_display.id)).is_some() {
                    draw_rectangle_lines(
                        cell_display.rect.x,
                        cell_display.rect.y,
                        cell_display.rect.w,
                        cell_display.rect.h,
                        5.0,
                        GREEN,
                    );
                }
            }
        }
    }

    async fn render_realm(&self) {
        for card_rect in &self.card_rects {
            if !matches!(card_rect.zone, Zone::Realm(_)) {
                continue;
            }

            let mut rotation = 0.0;
            if card_rect.tapped {
                rotation = std::f32::consts::FRAC_PI_2;
            }

            let rect = card_rect.rect;
            draw_texture_ex(
                &card_rect.image,
                rect.x,
                rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(rect.w, rect.h) * CARD_IN_PLAY_SCALE),
                    rotation,
                    ..Default::default()
                },
            );

            let mut sleeve_color = DARKGREEN;
            if card_rect.owner_id != self.player_id {
                sleeve_color = RED;
            }

            // Draw rectangle border rotated around the center
            let w = rect.w * CARD_IN_PLAY_SCALE;
            let h = rect.h * CARD_IN_PLAY_SCALE;
            let cx = rect.x + w / 2.0;
            let cy = rect.y + h / 2.0;
            let corners = [
                Vec2::new(-w / 2.0, -h / 2.0),
                Vec2::new(w / 2.0, -h / 2.0),
                Vec2::new(w / 2.0, h / 2.0),
                Vec2::new(-w / 2.0, h / 2.0),
            ];
            let rotated: Vec<Vec2> = corners
                .iter()
                .map(|corner| {
                    let (sin, cos) = rotation.sin_cos();
                    Vec2::new(
                        cos * corner.x - sin * corner.y + cx,
                        sin * corner.x + cos * corner.y + cy,
                    )
                })
                .collect();
            for i in 0..4 {
                draw_line(
                    rotated[i].x,
                    rotated[i].y,
                    rotated[(i + 1) % 4].x,
                    rotated[(i + 1) % 4].y,
                    2.0,
                    sleeve_color,
                );
            }

            if let Status::SelectingCard { cards } = &self.status {
                if !cards.contains(&card_rect.id) {
                    draw_rectangle_ex(
                        rect.x,
                        rect.y,
                        rect.w * CARD_IN_PLAY_SCALE,
                        rect.h * CARD_IN_PLAY_SCALE,
                        DrawRectangleParams {
                            color: Color::new(100.0, 100.0, 100.0, 0.6),
                            rotation,
                            ..Default::default()
                        },
                    );
                }
            }

            if card_rect.modifiers.contains(&Modifier::SummoningSickness) {
                let icon_size = 22.0;
                let scale = CARD_IN_PLAY_SCALE;
                let x = card_rect.rect.x + card_rect.rect.w * scale - icon_size - 4.0;
                let y = card_rect.rect.y + 4.0;
                draw_vortex_icon(x, y, icon_size, BLUE);
            }

            if card_rect.modifiers.contains(&Modifier::Disabled) {
                let icon_size = 15.0;
                let x = card_rect.rect.x + card_rect.rect.w - 30.0 - 5.0;
                let y = card_rect.rect.y + 4.0;
                let cx = x + icon_size / 2.0;
                let cy = y + icon_size / 2.0;
                draw_circle_lines(cx, cy, icon_size / 2.0, 3.0, WHITE);
                draw_line(x + 4.0, y + icon_size - 4.0, x + icon_size - 4.0, y + 4.0, 3.0, WHITE);
            }
        }
    }

    async fn render_player_hand(&self) {
        let hand_rect = hand_rect();
        let bg_color = Color::new(0.15, 0.18, 0.22, 0.85);
        draw_rectangle(hand_rect.x, hand_rect.y, hand_rect.w, hand_rect.h, bg_color);

        for card_rect in &self.card_rects {
            if card_rect.zone != Zone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_rect.is_selected || card_rect.is_hovered {
                scale = 1.2;
            }

            let rect = card_rect.rect;
            draw_texture_ex(
                &card_rect.image,
                rect.x,
                rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(rect.w, rect.h) * scale),
                    rotation: card_rect.rotation.clone(),
                    ..Default::default()
                },
            );

            draw_rectangle_lines(rect.x, rect.y, rect.w * scale, rect.h * scale, 5.0, DARKGREEN);

            if let Status::SelectingCard { cards } = &self.status {
                if !cards.contains(&card_rect.id) {
                    draw_rectangle_ex(
                        rect.x,
                        rect.y,
                        rect.w * scale,
                        rect.h * scale,
                        DrawRectangleParams {
                            color: Color::new(200.0, 200.0, 200.0, 0.6),
                            rotation: card_rect.rotation,
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    async fn render_background(&self) {
        let rect = realm_rect();
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.08, 0.12, 0.18, 1.0));
    }

    async fn compute_realm_rects(&mut self) -> anyhow::Result<()> {
        for card in &self.cards {
            if let Zone::Realm(square) = card.zone {
                let cell_rect = self.cell_rects.iter().find(|c| c.id == square).unwrap().rect;
                let mut dimensions = spell_dimensions();
                if card.card_type == CardType::Site {
                    dimensions = site_dimensions();
                }

                let rect = Rect::new(
                    cell_rect.x + (cell_rect.w - dimensions.x) / 2.0,
                    cell_rect.y + (cell_rect.h - dimensions.y) / 2.0,
                    dimensions.x,
                    dimensions.y,
                );

                self.card_rects.push(CardRect {
                    id: card.id,
                    owner_id: card.owner_id,
                    zone: card.zone.clone(),
                    tapped: card.tapped,
                    image: TextureCache::get_card_texture(&card).await,
                    rect,
                    rotation: 0.0,
                    is_hovered: false,
                    is_selected: false,
                    modifiers: card.modifiers.clone(),
                });
            }
        }

        Ok(())
    }

    async fn compute_hand_rects(&mut self) -> anyhow::Result<()> {
        let spells: Vec<&RenderableCard> = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_spell())
            .collect();

        let sites: Vec<&RenderableCard> = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_site())
            .collect();

        let spell_hand_size = spells.len();
        let site_hand_size = sites.len();
        let mut displays: Vec<CardRect> = Vec::new();
        let hand_rect = hand_rect();

        // Layout parameters
        let spell_dim = spell_dimensions();
        let site_dim = site_dimensions();
        let card_spacing = 20.0;

        // Calculate total width for spells (horizontal row)
        let spells_width = if spell_hand_size > 0 {
            spell_hand_size as f32 * spell_dim.x + (spell_hand_size as f32 - 1.0) * card_spacing
        } else {
            0.0
        };

        // Combined width for centering: spells row + spacing + site card width (if any sites)
        let total_width = spells_width
            + if site_hand_size > 0 {
                card_spacing + site_dim.x
            } else {
                0.0
            };

        // Center horizontally in hand area
        let start_x = hand_rect.x + (hand_rect.w - total_width) / 2.0;
        let spells_y = hand_rect.y + hand_rect.h / 2.0 - spell_dim.y / 2.0;

        // Spells row
        for (idx, card) in spells.iter().enumerate() {
            let x = start_x + idx as f32 * (spell_dim.x + card_spacing);
            let rect = Rect::new(x, spells_y, spell_dim.x, spell_dim.y);

            displays.push(CardRect {
                id: card.id,
                owner_id: card.owner_id,
                rect,
                is_hovered: false,
                is_selected: false,
                rotation: 0.0,
                zone: card.zone.clone(),
                tapped: card.tapped,
                image: TextureCache::get_card_texture(card).await,
                modifiers: card.modifiers.clone(),
            });
        }

        // Sites column, stacked vertically to the right of spells
        if site_hand_size > 0 {
            let sites_x = start_x + spells_width + card_spacing;
            let sites_start_y = hand_rect.y + hand_rect.h / 2.0 - spell_dim.y / 2.0;
            for (idx, card) in sites.iter().enumerate() {
                let y = sites_start_y + idx as f32 * 20.0;
                let rect = Rect::new(sites_x, y, site_dim.x, site_dim.y);

                displays.push(CardRect {
                    id: card.id,
                    owner_id: card.owner_id,
                    rect,
                    is_hovered: false,
                    is_selected: false,
                    rotation: 0.0,
                    zone: card.zone.clone(),
                    tapped: card.tapped,
                    image: TextureCache::get_card_texture(card).await,
                    modifiers: card.modifiers.clone(),
                });
            }
        }

        self.card_rects = displays;
        Ok(())
    }
}
