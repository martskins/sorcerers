use crate::{
    components::{Component, player_hand::PlayerHandComponent, realm::RealmComponent},
    config::*,
    render::{CardRect, CellRect, IntersectionRect},
    scene::{Scene, selection_overlay::SelectionOverlay},
    set_clicks_enabled,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{BLUE, Color, RED, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{RectOffset, Vec2},
    shapes::{draw_line, draw_rectangle, draw_triangle_lines},
    text::draw_text,
    texture::{DrawTextureParams, draw_texture_ex},
    ui::{self, hash},
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::{CardType, Plane, RenderableCard, Zone},
    game::{Element, PlayerId, Resources},
    networking::{
        self,
        message::{ClientMessage, ServerMessage},
    },
};
use std::collections::HashMap;

use super::selection_overlay::SelectionOverlayBehaviour;

const FONT_SIZE: f32 = 24.0;
const THRESHOLD_SYMBOL_SPACING: f32 = 18.0;
const SYMBOL_SIZE: f32 = 20.0;

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Idle,
    SelectingAction {
        prompt: String,
    },
    SelectingCard {
        cards: Vec<uuid::Uuid>,
        preview: bool,
        prompt: String,
    },
    SelectingZone {
        zones: Vec<Zone>,
    },
    ViewingCards {
        cards: Vec<uuid::Uuid>,
        prompt: String,
        prev_status: Box<Status>,
        behaviour: SelectionOverlayBehaviour,
    },
}

#[derive(Debug)]
pub struct Game {
    pub player_id: PlayerId,
    pub opponent_id: PlayerId,
    pub game_id: uuid::Uuid,
    pub card_rects: Vec<CardRect>,
    pub cell_rects: Vec<CellRect>,
    pub intersection_rects: Vec<IntersectionRect>,
    pub cards: Vec<RenderableCard>,
    pub resources: HashMap<PlayerId, Resources>,
    pub client: networking::client::Client,
    pub current_player: PlayerId,
    pub is_player_one: bool,
    card_selection_overlay: Option<SelectionOverlay>,
    actions: Vec<String>,
    status: Status,
    components: Vec<Box<dyn Component>>,
}

impl Game {
    pub fn new(
        game_id: uuid::Uuid,
        player_id: uuid::Uuid,
        opponent_id: uuid::Uuid,
        is_player_one: bool,
        cards: Vec<RenderableCard>,
        client: networking::client::Client,
    ) -> Self {
        let cell_rects: Vec<CellRect> = (0..20)
            .map(|i| {
                let rect = cell_rect(i + 1, !is_player_one);
                CellRect { id: i as u8 + 1, rect }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    let rect = intersection_rect(&locs, !is_player_one).unwrap();
                    Some(IntersectionRect { locations: locs, rect })
                }
                _ => None,
            })
            .collect();

        Self {
            player_id: player_id.clone(),
            opponent_id,
            card_rects: Vec::new(),
            cards,
            game_id: game_id.clone(),
            cell_rects,
            intersection_rects,
            client: client.clone(),
            current_player: uuid::Uuid::nil(),
            is_player_one,
            resources: HashMap::new(),
            actions: Vec::new(),
            card_selection_overlay: None,
            status: Status::Idle,
            components: vec![
                Box::new(PlayerHandComponent::new(&game_id, &player_id, client.clone())),
                Box::new(RealmComponent::new(
                    &game_id,
                    &player_id,
                    !is_player_one,
                    client.clone(),
                )),
            ],
        }
    }

    fn is_players_turn(&self, player_id: &PlayerId) -> bool {
        self.current_player == *player_id
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        for component in &mut self.components {
            component.update(&self.cards, self.status.clone()).await?;
        }

        // Update click_enabled at the end of the update cycle so that we don't process the release
        // event in the same frame as the one that re-enables clicking.
        if is_mouse_button_released(MouseButton::Left) {
            set_clicks_enabled(true);
        }

        if let Status::ViewingCards {
            cards,
            behaviour,
            prev_status,
            prompt,
        } = &self.status
        {
            let renderables = self.cards.iter().filter(|c| cards.contains(&c.id)).collect();
            self.card_selection_overlay = Some(
                SelectionOverlay::new(
                    self.client.clone(),
                    &self.game_id,
                    &self.player_id,
                    renderables,
                    prompt,
                    behaviour.clone(),
                )
                .await,
            );
            self.status = *prev_status.clone();
        }

        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.update();

            if overlay.should_close() {
                self.card_selection_overlay = None;
                self.status = Status::Idle;
            }
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

        self.render_gui().await?;
        for component in &mut self.components {
            component.render(&mut self.status).await;
        }

        self.render_card_preview().await?;
        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.render();
        }
        Ok(())
    }

    pub async fn process_message(&mut self, message: &ServerMessage) -> anyhow::Result<Option<Scene>> {
        match message {
            ServerMessage::LogEvent { description } => {
                println!("Game Log: {}", description);
                Ok(None)
            }
            ServerMessage::PickZone { zones, .. } => {
                self.status = Status::SelectingZone { zones: zones.clone() };
                Ok(None)
            }
            ServerMessage::PickCard {
                cards, prompt, preview, ..
            } => {
                self.status = Status::SelectingCard {
                    cards: cards.clone(),
                    preview: preview.clone(),
                    prompt: prompt.clone(),
                };

                if *preview {
                    let renderables = self.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                    self.card_selection_overlay = Some(
                        SelectionOverlay::new(
                            self.client.clone(),
                            &self.game_id,
                            &self.player_id,
                            renderables,
                            prompt,
                            SelectionOverlayBehaviour::Pick,
                        )
                        .await,
                    );
                }
                Ok(None)
            }
            ServerMessage::PickAction { prompt, actions, .. } => {
                self.actions = actions.clone();
                self.status = Status::SelectingAction {
                    prompt: prompt.to_string(),
                };
                Ok(None)
            }
            ServerMessage::GameStarted {
                game_id,
                player1,
                player2,
                cards,
                ..
            } => {
                // Flip the board for player 2. Use player1 instead of the is_player_one method
                // because state is not set at this point.
                self.is_player_one = player1 == &self.player_id;
                self.opponent_id = if self.is_player_one {
                    player2.clone()
                } else {
                    player1.clone()
                };
                if !self.is_player_one {
                    for cell in &mut self.cell_rects {
                        let new_id: i8 = cell.id as i8 - 21;
                        cell.id = new_id.abs() as u8;
                    }
                }

                let intersection_rects = Zone::all_intersections()
                    .into_iter()
                    .filter_map(|z| match z {
                        Zone::Intersection(locs) => {
                            let rect = intersection_rect(&locs, !self.is_player_one).unwrap();
                            Some(IntersectionRect { locations: locs, rect })
                        }
                        _ => None,
                    })
                    .collect();
                self.intersection_rects = intersection_rects;
                self.game_id = game_id.clone();
                // TODO: Fix so client doesn't hang
                TextureCache::load_cache(cards).await;
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

    pub fn process_input(&mut self) {
        if self.card_selection_overlay.is_some() {
            let overlay = self.card_selection_overlay.as_mut().unwrap();
            overlay.process_input();
            return;
        }

        for component in &mut self.components {
            component.process_input(self.current_player == self.player_id, &mut self.status);
        }
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

    async fn render_player_card(&self, x: f32, y: f32, player_id: &uuid::Uuid) {
        let resources = self.resources.get(&self.player_id).cloned().unwrap_or(Resources::new());
        let player_name = if &self.player_id == player_id { "You" } else { "Them" };
        draw_text(player_name, x, y, FONT_SIZE, WHITE);

        const ICON_SIZE: f32 = 20.0;
        const NAME_BOTTOM_MARGIN: f32 = 7.0;
        let icon_y = y + NAME_BOTTOM_MARGIN;
        let health_text_y: f32 = y + NAME_BOTTOM_MARGIN + 20.0;
        let heart_texture = TextureCache::get_texture("assets/icons/heart.png").await;
        draw_texture_ex(
            &heart_texture,
            x,
            icon_y + 5.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE - 5.0, ICON_SIZE - 5.0)),
                ..Default::default()
            },
        );
        let health = format!("{}", resources.health);
        draw_text(&health, x + 22.0, health_text_y, FONT_SIZE, WHITE);

        let cards_texture = TextureCache::get_texture("assets/icons/cards.png").await;
        draw_texture_ex(
            &cards_texture,
            x + 52.0,
            icon_y + 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let cards_in_hand = format!(
            "{}",
            self.cards
                .iter()
                .filter(|c| &c.owner_id == player_id)
                .filter(|c| c.zone == Zone::Hand)
                .count()
        );
        draw_text(&cards_in_hand, x + 77.0, health_text_y, FONT_SIZE, WHITE);

        let potion_texture = TextureCache::get_texture("assets/icons/potion.png").await;
        draw_texture_ex(
            &potion_texture,
            x + 95.0,
            icon_y + 4.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let mana_text = format!("{}", resources.mana);
        draw_text(&mana_text, x + 120.0, health_text_y, FONT_SIZE, WHITE);

        let tombstone_texture = TextureCache::get_texture("assets/icons/tombstone.png").await;
        draw_texture_ex(
            &tombstone_texture,
            x + 140.0,
            icon_y + 5.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(ICON_SIZE, ICON_SIZE)),
                ..Default::default()
            },
        );
        let cards_in_cemetery = format!(
            "{}",
            self.cards
                .iter()
                .filter(|c| &c.owner_id == player_id)
                .filter(|c| c.zone == Zone::Cemetery)
                .count()
        );
        draw_text(&cards_in_cemetery, x + 165.0, health_text_y, FONT_SIZE, WHITE);

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
        const BASE_X: f32 = 20.0;
        let player_y: f32 = screen_rect.h - 90.0;
        self.render_player_card(BASE_X, player_y, &self.player_id).await;

        const OPPONENT_Y: f32 = 25.0;
        self.render_player_card(BASE_X, OPPONENT_Y, &self.opponent_id).await;

        let turn_label = if self.is_players_turn(&self.player_id) {
            "Your Turn"
        } else {
            "Opponent's Turn"
        };

        draw_text(turn_label, screen_rect.w / 2.0 - 50.0, 30.0, FONT_SIZE, WHITE);

        let is_in_turn = self.current_player == self.player_id;
        let is_idle = matches!(self.status, Status::Idle);
        if is_in_turn && is_idle {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 100.0, screen_rect.h - 40.0), "Pass Turn") {
                set_clicks_enabled(false);
                self.client.send(ClientMessage::EndTurn {
                    player_id: self.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        }

        match self.status {
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
                                set_clicks_enabled(false);
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
        if let Status::SelectingCard { preview: true, .. } = &self.status {
            return Ok(());
        }

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

    pub fn wrap_text(text: &str, max_width: f32, font_size: u16) -> String {
        use macroquad::text::measure_text;
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let test = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current, word)
                };
                let dims = measure_text(&test, None, font_size, 1.0);
                if dims.width > max_width && !current.is_empty() {
                    lines.push(current.clone());
                    current = word.to_string();
                } else {
                    current = test;
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
        }
        lines.join("\n")
    }
}
