use crate::{
    config::*,
    render::{CardDisplay, CellDisplay},
    scene::Scene,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{BLUE, Color, GREEN, RED, WHITE},
    input::{MouseButton, is_mouse_button_released, mouse_position},
    math::{Rect, Vec2},
    shapes::{draw_circle_lines, draw_line, draw_rectangle, draw_rectangle_lines, draw_triangle_lines},
    text::draw_text,
    texture::{DrawTextureParams, draw_texture_ex},
    ui,
};
use sorcerers::{
    card::{CardInfo, CardType, Modifier, Plane, Zone},
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
const ACTION_SELECTION_WINDOW_ID: u64 = 1;

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
    None,
    SelectingAction,
    DrawCard,
    SelectingCard { cards: Vec<uuid::Uuid> },
    SelectingZone { zones: Vec<Zone> },
}

#[derive(Debug)]
pub struct Game {
    pub player_id: PlayerId,
    pub game_id: uuid::Uuid,
    pub card_displays: Vec<CardDisplay>,
    pub cells: Vec<CellDisplay>,
    pub cards: Vec<CardInfo>,
    pub resources: HashMap<PlayerId, Resources>,
    pub client: networking::client::Client,
    pub current_player: PlayerId,
    pub is_player_one: bool,
    action_window_position: Option<Vec2>,
    action_window_size: Option<Vec2>,
    actions: Vec<String>,
    status: Status,
}

impl Game {
    pub fn new(player_id: uuid::Uuid, client: networking::client::Client) -> Self {
        let cells = (0..20)
            .map(|i| {
                let rect = cell_rect(i + 1, false);
                CellDisplay { id: i as u8 + 1, rect }
            })
            .collect();
        Self {
            player_id: player_id,
            card_displays: Vec::new(),
            cards: Vec::new(),
            game_id: uuid::Uuid::nil(),
            cells,
            client,
            current_player: uuid::Uuid::nil(),
            is_player_one: false,
            resources: HashMap::new(),
            action_window_position: None,
            action_window_size: None,
            actions: Vec::new(),
            status: Status::None,
        }
    }

    fn is_players_turn(&self, player_id: &PlayerId) -> bool {
        self.current_player == *player_id
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        let mouse_position = mouse_position().into();
        self.resize_cells().await?;
        self.update_cards_in_hand().await?;
        self.update_cards_in_realm().await?;
        self.handle_click(mouse_position);
        Ok(())
    }

    async fn resize_cells(&mut self) -> anyhow::Result<()> {
        let mirror = !self.is_player_one;
        for cell in &mut self.cells {
            cell.rect = cell_rect(cell.id, mirror);
        }

        Ok(())
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        self.render_background().await;
        self.render_grid().await;
        self.render_deck().await;
        self.render_player_hand().await;
        self.render_realm().await;
        self.render_cemetery().await?;
        self.render_gui().await?;
        self.render_card_preview().await?;
        Ok(())
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        if is_mouse_button_released(MouseButton::Left) {
            let mouse_position = mouse_position();
            if atlasbook_rect().contains(mouse_position.into()) {
                self.draw_card(CardType::Site).unwrap();
            }

            if spellbook_rect().contains(mouse_position.into()) {
                self.draw_card(CardType::Spell).unwrap();
            }
        }

        None
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
            ServerMessage::PickAction { actions, .. } => {
                self.actions = actions.clone();
                self.status = Status::SelectingAction;
                Ok(None)
            }
            ServerMessage::GameStarted { game_id, player1, .. } => {
                // Flip the board for player 2. Use player1 instead of the is_player_one method
                // because state is not set at this point.
                self.is_player_one = player1 == &self.player_id;
                if !self.is_player_one {
                    for cell in &mut self.cells {
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

        if element == Element::Air || element == Element::Water {
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

    fn render_resources(x: f32, y: f32, resources: &Resources) {
        let health = format!("Health: {}", resources.health);
        draw_text(&health, x, y, FONT_SIZE, WHITE);

        let mana_text = format!("Mana: {}", resources.mana);
        draw_text(&mana_text, x, y + 20.0, FONT_SIZE, WHITE);

        let thresholds_y: f32 = y + 10.0 + 20.0;
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
        let player_y: f32 = screen_rect.h - 70.0;
        let resources = self.resources.get(&self.player_id).cloned().unwrap_or(Resources::new());
        Game::render_resources(base_x, player_y, &resources);

        const OPPONENT_Y: f32 = 25.0;
        let opponent_resources = self
            .resources
            .iter()
            .find(|(player_id, _)| **player_id != self.player_id)
            .map(|(_, resources)| resources.clone())
            .unwrap_or(Resources::new());
        Game::render_resources(base_x, OPPONENT_Y, &opponent_resources);

        let turn_label = if self.is_players_turn(&self.player_id) {
            if let Status::DrawCard { .. } = self.status {
                "Draw a Card"
            } else {
                "Your Turn"
            }
        } else {
            "Opponent's Turn"
        };

        let player1 = if self.is_player_one { "Player 1" } else { "Player 2" };
        draw_text(player1, screen_rect.w / 2.0 - 150.0, 30.0, FONT_SIZE, WHITE);
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
            Status::SelectingCard { ref cards } => {
                let valid_cards: Vec<&CardDisplay> =
                    self.card_displays.iter().filter(|c| cards.contains(&c.id)).collect();

                for card in valid_cards {
                    draw_rectangle_lines(card.rect.x, card.rect.y, card.rect.w, card.rect.h, 3.0, WHITE);
                }
            }
            Status::SelectingAction => {
                if self.action_window_position.is_none() {
                    self.action_window_position = Some(Vec2::new(0.0, 0.0).into());
                }

                if self.action_window_size.is_none() {
                    self.action_window_size = Some(Vec2::new(100.0, 30.0 * self.actions.len() as f32 + 20.0));
                }

                ui::root_ui().window(
                    ACTION_SELECTION_WINDOW_ID,
                    self.action_window_position.unwrap(),
                    self.action_window_size.unwrap(),
                    |ui| {
                        for (idx, action) in self.actions.iter().enumerate() {
                            if ui.button(Vec2::new(0.0, 30.0 * idx as f32), action.as_str()) {
                                self.client
                                    .send(ClientMessage::PickAction {
                                        game_id: self.game_id,
                                        player_id: self.player_id,
                                        action_idx: idx,
                                    })
                                    .unwrap();
                                self.status = Status::None;
                            }
                        }
                    },
                );
            }
            _ => {}
        }

        // match &self.player_status {
        //     Status::SelectingDirection { player_id, directions } if player_id == &self.player_id => {
        //         if self.action_window_position.is_none() {
        //             // self.action_window_position = Some(mouse_position().into());
        //             self.action_window_position = Some(Vec2::new(0.0, 0.0).into());
        //         }
        //
        //         if self.action_window_size.is_none() {
        //             self.action_window_size = Some(Vec2::new(100.0, 30.0 * directions.len() as f32 + 20.0));
        //         }
        //
        //         ui::root_ui().window(
        //             ACTION_SELECTION_WINDOW_ID,
        //             self.action_window_position.unwrap(),
        //             self.action_window_size.unwrap(),
        //             |ui| {
        //                 for (idx, direction) in directions.iter().enumerate() {
        //                     if ui.button(Vec2::new(0.0, 30.0 * idx as f32), direction.get_name()) {
        //                         println!("Picked direction: {:?}", direction);
        //                         let direction = directions[idx].normalise(!self.is_player_one);
        //                         println!("Normalised direction: {:?}", direction);
        //                         self.client
        //                             .send(ClientMessage::PickDirection {
        //                                 player_id: self.player_id,
        //                                 game_id: self.game_id,
        //                                 direction,
        //                             })
        //                             .unwrap();
        //                     }
        //                 }
        //             },
        //         );
        //     }
        //     Status::SelectingAction { player_id, actions } if player_id == &self.player_id => {
        //         if self.action_window_position.is_none() {
        //             // self.action_window_position = Some(mouse_position().into());
        //             self.action_window_position = Some(Vec2::new(0.0, 0.0).into());
        //         }
        //
        //         if self.action_window_size.is_none() {
        //             self.action_window_size = Some(Vec2::new(100.0, 30.0 * actions.len() as f32 + 20.0));
        //         }
        //
        //         ui::root_ui().window(
        //             ACTION_SELECTION_WINDOW_ID,
        //             self.action_window_position.unwrap(),
        //             self.action_window_size.unwrap(),
        //             |ui| {
        //                 for (idx, action) in actions.iter().enumerate() {
        //                     if ui.button(Vec2::new(0.0, 30.0 * idx as f32), action.to_string()) {
        //                         self.client
        //                             .send(ClientMessage::PickAction {
        //                                 action_idx: idx,
        //                                 player_id: self.player_id,
        //                                 game_id: self.game_id,
        //                             })
        //                             .unwrap();
        //                     }
        //                 }
        //             },
        //         );
        //     }
        //     Status::SelectingCard {
        //         player_id, valid_cards, ..
        //     } if player_id == &self.player_id => {
        //         let valid_cards: Vec<&CardDisplay> = self
        //             .card_displays
        //             .iter()
        //             .filter(|c| valid_cards.contains(&c.id))
        //             .collect();
        //
        //         let mut scale = 1.0;
        //         for card in valid_cards {
        //             if card.is_hovered {
        //                 scale = 1.2;
        //             }
        //
        //             draw_rectangle_lines(
        //                 card.rect.x,
        //                 card.rect.y,
        //                 card.rect.w * scale,
        //                 card.rect.h * scale,
        //                 3.0,
        //                 WHITE,
        //             );
        //         }
        //     }
        //     _ => {} //
        //             // PlayerStatus::SelectingCardOutsideRealm {
        //             //     player_id,
        //             //     owner,
        //             //     spellbook,
        //             //     cemetery,
        //             //     hand,
        //             //     after_select,
        //             // } => {
        //             //     if player_id != &self.player_id {
        //             //         return Ok(());
        //             //     }
        //             //
        //             //     let mut number_of_zones = 0;
        //             //     if spellbook.is_some() {
        //             //         number_of_zones += 1;
        //             //     }
        //             //
        //             //     if hand.is_some() {
        //             //         number_of_zones += 1;
        //             //     }
        //             //
        //             //     if cemetery.is_some() {
        //             //         number_of_zones += 1;
        //             //     }
        //             //
        //             //     let width = screen_width() - 20.0;
        //             //     let height = screen_height() - 20.0;
        //             //     let mut images = HashMap::new();
        //             //     let cards_to_display: Vec<CardDisplay> = self
        //             //         .card_displays
        //             //         .iter()
        //             //         .filter(|c| {
        //             //             if owner.is_some() {
        //             //                 return c.owner_id == owner.as_ref().unwrap();
        //             //             }
        //             //
        //             //             true
        //             //         })
        //             //         .map(|c| c.card.clone())
        //             //         .collect();
        //             //
        //             //     for card in &cards_to_display {
        //             //         images.insert(card.name, card.image);
        //             //     }
        //             //
        //             //     let mut all_valid_cards: Vec<uuid::Uuid> = spellbook.as_deref().unwrap_or_default().into();
        //             //     all_valid_cards.extend(cemetery.as_deref().unwrap_or_default());
        //             //     all_valid_cards.extend(hand.as_deref().unwrap_or_default());
        //             //     ui::root_ui().window(
        //             //         CARD_SELECTION_WINDOW_ID,
        //             //         Vec2::new(10.0, 10.0),
        //             //         Vec2::new(width, height),
        //             //         |ui| {
        //             //             for (idx, card) in cards_to_display.iter().enumerate() {
        //             //                 if Texture::new(images.get(card.get_name()).unwrap().clone())
        //             //                     .position(Vec2::new(10.0, 30.0 * idx as f32))
        //             //                     .size(card_width(), card_height())
        //             //                     .ui(ui)
        //             //                 {
        //             //                     self.client
        //             //                         .send(Message::SummonMinion {
        //             //                             player_id: self.player_id.clone(),
        //             //                             card_id: card.get_id().clone(),
        //             //                             game_id: self.game_id.clone(),
        //             //                             square: 0,
        //             //                         })
        //             //                         .unwrap();
        //             //                 }
        //             //
        //             //                 // if spellbook.as_deref().unwrap_or_default().contains(card.get_id()) {
        //             //                 //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
        //             //                 // }
        //             //                 //
        //             //                 // if cemetery.as_deref().unwrap_or_default().contains(card.get_id()) {
        //             //                 //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
        //             //                 // }
        //             //                 //
        //             //                 // if hand.as_deref().unwrap_or_default().contains(card.get_id()) {
        //             //                 //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
        //             //                 // }
        //             //             }
        //             //         },
        //             //     );
        //             // }
        //
        //             // _ => {}
        // }

        Ok(())
    }

    async fn render_card_preview(&self) -> anyhow::Result<()> {
        let selected_card = self.card_displays.iter().find(|card_display| card_display.is_hovered);
        let screen_rect = screen_rect();

        if let Some(card_display) = selected_card {
            const PREVIEW_SCALE: f32 = 2.7;
            let mut rect = card_display.rect;
            rect.w *= PREVIEW_SCALE;
            rect.h *= PREVIEW_SCALE;
            // let mut dimensions = spell_dimensions() * PREVIEW_SCALE;
            // if card_display.is_site {
            //     dimensions = site_dimensions() * PREVIEW_SCALE;
            // }

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
            .card_displays
            .iter()
            .filter(|card_display| card_display.zone == Zone::Cemetery)
            .collect::<Vec<&CardDisplay>>();
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
        let mut hovered_card_index = None;
        for (idx, card_display) in self.card_displays.iter().enumerate() {
            if card_display.rect.contains(mouse_position.into()) {
                hovered_card_index = Some(idx);
            };
        }

        for card in &mut self.card_displays {
            card.is_hovered = false;
        }

        if let Some(idx) = hovered_card_index {
            self.card_displays.get_mut(idx).unwrap().is_hovered = true;
        }

        match &self.status {
            Status::None => {
                let mut card_selected = None;
                for card_display in &mut self
                    .card_displays
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
                let valid_cards: Vec<&CardDisplay> =
                    self.card_displays.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
                    let card = self.card_displays.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    println!("Card selected: {:?}", card.is_selected);
                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        self.status = Status::None;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_square_click(&mut self, mouse_position: Vec2) {
        match &self.status {
            Status::SelectingZone { zones } => {
                let zones = zones.clone();
                for (idx, cell) in self.cells.iter().enumerate() {
                    if zones.iter().find(|i| i == &&Zone::Realm(cell.id)).is_none() {
                        continue;
                    }

                    if cell.rect.contains(mouse_position.into()) {
                        let square = self.cells[idx].id;
                        if is_mouse_button_released(MouseButton::Left) {
                            self.client
                                .send(ClientMessage::PickSquare {
                                    player_id: self.player_id.clone(),
                                    game_id: self.game_id.clone(),
                                    square,
                                })
                                .unwrap();

                            self.status = Status::None;
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
        for cell_display in &self.cells {
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
                WHITE,
            );

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
        for card_display in &self.card_displays {
            if !matches!(card_display.zone, Zone::Realm(_)) {
                continue;
            }

            let mut rotation = 0.0;
            if card_display.tapped {
                rotation = std::f32::consts::FRAC_PI_2;
            }

            draw_texture_ex(
                &card_display.image,
                card_display.rect.x,
                card_display.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(card_display.rect.w, card_display.rect.h) * CARD_IN_PLAY_SCALE),
                    rotation,
                    ..Default::default()
                },
            );

            if card_display.modifiers.contains(&Modifier::SummoningSickness) {
                let icon_size = 22.0;
                let scale = CARD_IN_PLAY_SCALE;
                let x = card_display.rect.x + card_display.rect.w * scale - icon_size - 4.0;
                let y = card_display.rect.y + 4.0;
                draw_vortex_icon(x, y, icon_size, BLUE);
            }

            if card_display.modifiers.contains(&Modifier::Disabled) {
                let icon_size = 15.0;
                let x = card_display.rect.x + card_display.rect.w - 30.0 - 5.0;
                let y = card_display.rect.y + 4.0;
                let cx = x + icon_size / 2.0;
                let cy = y + icon_size / 2.0;
                draw_circle_lines(cx, cy, icon_size / 2.0, 3.0, WHITE);
                draw_line(x + 4.0, y + icon_size - 4.0, x + icon_size - 4.0, y + 4.0, 3.0, WHITE);
            }
        }
    }

    async fn render_player_hand(&self) {
        for card_display in &self.card_displays {
            if card_display.zone != Zone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_display.is_selected || card_display.is_hovered {
                scale = 1.2;
            }

            draw_texture_ex(
                &card_display.image,
                card_display.rect.x,
                card_display.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(card_display.rect.w, card_display.rect.h) * scale),
                    rotation: card_display.rotation,
                    ..Default::default()
                },
            );
        }
    }

    async fn render_background(&self) {
        let rect = realm_rect();
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.08, 0.12, 0.18, 1.0));
    }

    async fn render_deck(&self) {
        let spellbook_rect = spellbook_rect();
        draw_texture_ex(
            &TextureCache::get_texture(SPELLBOOK_IMAGE, "spellbook").await,
            spellbook_rect.x,
            spellbook_rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(spellbook_rect.size() * CARD_IN_PLAY_SCALE),
                ..Default::default()
            },
        );

        let atlasbook_rect = atlasbook_rect();
        draw_texture_ex(
            &TextureCache::get_texture(ATLASBOOK_IMAGE, "atlas").await,
            atlasbook_rect.x,
            atlasbook_rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(atlasbook_rect.size() * CARD_IN_PLAY_SCALE),
                ..Default::default()
            },
        );
    }

    fn draw_card(&self, card_type: CardType) -> anyhow::Result<()> {
        if Status::DrawCard == self.status {
            self.client
                .send(ClientMessage::DrawCard {
                    card_type,
                    player_id: self.player_id,
                    game_id: self.game_id,
                })
                .unwrap();
        }

        Ok(())
    }

    async fn update_cards_in_realm(&mut self) -> anyhow::Result<()> {
        for card in &self.cards {
            if let Zone::Realm(square) = card.zone {
                let cell_rect = self.cells.iter().find(|c| c.id == square).unwrap().rect;
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

                self.card_displays.push(CardDisplay {
                    id: card.id,
                    zone: card.zone.clone(),
                    tapped: card.tapped,
                    image: TextureCache::get_card_texture(&card).await,
                    rect,
                    rotation: 0.0,
                    is_hovered: false,
                    is_selected: false,
                    modifiers: card.modifiers.clone(),
                    plane: card.plane.clone(),
                    card_type: card.card_type.clone(),
                });
            }
        }

        Ok(())
    }

    async fn update_cards_in_hand(&mut self) -> anyhow::Result<()> {
        let spells: Vec<&CardInfo> = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_spell())
            .collect();

        let sites: Vec<&CardInfo> = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_site())
            .collect();

        let spell_hand_size = spells.len();
        let fan_angle = 30.0;
        let center = spell_hand_size as f32 / 2.0;
        let radius = 40.0;

        let mut displays: Vec<CardDisplay> = Vec::new();
        let hand_rect = hand_rect();
        for (idx, card) in spells.iter().enumerate() {
            let dimensions = spell_dimensions();
            let angle = if spell_hand_size > 1 {
                ((idx as f32 - center) / center) * (fan_angle / 2.0)
            } else {
                0.0
            };
            let rad = angle.to_radians();
            let x = hand_rect.x + idx as f32 * CARD_OFFSET_X + 20.0;
            let y = hand_rect.y + 20.0 + radius * rad.sin();

            let rect = Rect::new(x, y, dimensions.x, dimensions.y);

            displays.push(CardDisplay {
                rect,
                is_hovered: false,
                is_selected: false,
                rotation: rad,
                id: card.id,
                zone: card.zone.clone(),
                tapped: card.tapped,
                image: TextureCache::get_card_texture(card).await,
                modifiers: card.modifiers.clone(),
                plane: card.plane.clone(),
                card_type: card.card_type.clone(),
            });
        }

        let site_x = hand_rect.x + spell_hand_size as f32 * CARD_OFFSET_X + 40.0;
        for (idx, card) in sites.iter().enumerate() {
            let dimensions = site_dimensions();
            let x = site_x;
            let y = hand_rect.y + 20.0 + 20.0 * idx as f32;

            let rect = Rect::new(x, y, dimensions.x, dimensions.y);

            displays.push(CardDisplay {
                rect,
                is_hovered: false,
                is_selected: false,
                rotation: 0.0,
                id: card.id,
                zone: card.zone.clone(),
                tapped: card.tapped,
                image: TextureCache::get_card_texture(card).await,
                modifiers: card.modifiers.clone(),
                plane: card.plane.clone(),
                card_type: card.card_type.clone(),
            });
        }

        self.card_displays = displays;
        Ok(())
    }
}
