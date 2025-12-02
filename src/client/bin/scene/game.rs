use std::collections::HashMap;

use macroquad::{
    color::{Color, BLUE, GREEN, RED, WHITE},
    input::{is_mouse_button_released, mouse_position, MouseButton},
    math::{Rect, Vec2},
    shapes::{draw_line, draw_rectangle, draw_rectangle_lines, draw_triangle_lines},
    text::draw_text,
    texture::{draw_texture_ex, DrawTextureParams},
    ui::{self, widgets::Texture},
    window::{screen_height, screen_width},
};
use sorcerers::{
    card::{Card, CardType, CardZone, Target},
    effect::{Action, GameAction},
    game::{Phase, Resources, State},
    networking::{self, Element, Message},
};

use crate::{
    config::*,
    render::{CardDisplay, CellDisplay},
    scene::Scene,
    texture_cache::TextureCache,
};

const FONT_SIZE: f32 = 24.0;
const THRESHOLD_SYMBOL_SPACING: f32 = 18.0;
const SYMBOL_SIZE: f32 = 20.0;
const ACTION_SELECTION_WINDOW_ID: u64 = 1;
const CARD_SELECTION_WINDOW_ID: u64 = 2;

#[derive(Debug)]
pub struct Game {
    pub player_id: uuid::Uuid,
    pub game_id: uuid::Uuid,
    pub cards: Vec<CardDisplay>,
    pub cells: Vec<CellDisplay>,
    pub client: networking::client::Client,
    pub state: State,
    action_window_position: Option<Vec2>,
    action_window_size: Option<Vec2>,
}

impl Game {
    pub fn new(player_id: uuid::Uuid, client: networking::client::Client) -> Self {
        let cells = (0..20)
            .map(|i| {
                let rect = cell_rect(i, true);
                CellDisplay { id: i as u8 + 1, rect }
            })
            .collect();
        Self {
            player_id,
            cards: vec![],
            game_id: uuid::Uuid::nil(),
            cells,
            client,
            state: State::new(vec![]),
            action_window_position: None,
            action_window_size: None,
        }
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        let mouse_position = mouse_position().into();
        let state = self.state.clone();
        self.resize_cells().await?;
        self.update_cards_in_hand(&state.cards)?;
        self.update_cards_in_realm(&state.cards).await?;
        self.handle_click(mouse_position);
        Ok(())
    }

    fn is_player_one(&self) -> bool {
        self.state.is_player_one(&self.player_id)
    }

    async fn resize_cells(&mut self) -> anyhow::Result<()> {
        let mirror = self.is_player_one();
        for cell in &mut self.cells {
            cell.rect = cell_rect(cell.id - 1, mirror);
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

    pub async fn process_message(&mut self, message: networking::Message) -> anyhow::Result<()> {
        match message {
            Message::MatchCreated { game_id, player1, .. } => {
                // Flip the board for player 2. Use player1 instead of the is_player_one method
                // because state is not set at this point.
                if player1 != self.player_id {
                    for cell in &mut self.cells {
                        let new_id: i8 = cell.id as i8 - 21;
                        cell.id = new_id.abs() as u8;
                    }
                }

                self.game_id = game_id;
                Ok(())
            }
            Message::Sync { state, .. } => {
                self.state = state.clone();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_click(&mut self, mouse_position: Vec2) {
        self.handle_card_click(mouse_position);
        self.handle_cell_click(mouse_position);
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
        Game::render_threshold(fire_x, fire_y, resources.fire_threshold, Element::Fire);

        let air_x = fire_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let air_y = thresholds_y;
        Game::render_threshold(air_x, air_y, resources.air_threshold, Element::Air);

        let earth_x = air_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let earth_y = thresholds_y;
        Game::render_threshold(earth_x, earth_y, resources.earth_threshold, Element::Earth);

        let water_x = earth_x + SYMBOL_SIZE + THRESHOLD_SYMBOL_SPACING + 5.0;
        let water_y = thresholds_y;
        Game::render_threshold(water_x, water_y, resources.water_threshold, Element::Water);
    }

    async fn render_gui(&mut self) -> anyhow::Result<()> {
        if self.state.phase == Phase::None {
            return Ok(());
        }

        let screen_rect = screen_rect();
        let base_x: f32 = 20.0;
        let player_y: f32 = screen_rect.h - 70.0;
        let resources = self
            .state
            .resources
            .get(&self.player_id)
            .cloned()
            .unwrap_or(Resources::new());
        Game::render_resources(base_x, player_y, &resources);

        const OPPONENT_Y: f32 = 25.0;
        let opponent_resources = self
            .state
            .resources
            .iter()
            .find(|(player_id, _)| **player_id != self.player_id)
            .map(|(_, resources)| resources.clone())
            .unwrap_or(Resources::new());
        Game::render_resources(base_x, OPPONENT_Y, &opponent_resources);

        let turn_label = if self.state.is_players_turn(&self.player_id) {
            "Your Turn"
        } else {
            "Opponent's Turn"
        };

        draw_text(turn_label, screen_rect.w / 2.0 - 50.0, 30.0, FONT_SIZE, WHITE);

        let is_in_turn = self.state.current_player == self.player_id;
        if is_in_turn {
            if ui::root_ui().button(Vec2::new(screen_rect.w - 100.0, screen_rect.h - 40.0), "End Turn") {
                self.client.send(Message::EndTurn {
                    player_id: self.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        }

        match &self.state.phase {
            Phase::SelectingAction { player_id, actions } if player_id == &self.player_id => {
                if self.action_window_position.is_none() {
                    self.action_window_position = Some(mouse_position().into());
                }

                if self.action_window_size.is_none() {
                    self.action_window_size = Some(Vec2::new(100.0, 30.0 * actions.len() as f32 + 20.0));
                }

                ui::root_ui().window(
                    ACTION_SELECTION_WINDOW_ID,
                    self.action_window_position.unwrap(),
                    self.action_window_size.unwrap(),
                    |ui| {
                        for (idx, action) in actions.iter().enumerate() {
                            if ui.button(Vec2::new(0.0, 30.0 * idx as f32), action.get_name()) {
                                self.client
                                    .send(Message::TriggerAction {
                                        action_idx: idx,
                                        player_id: self.player_id,
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                        }

                        if ui.button(Vec2::new(0.0, 30.0 * actions.len() as f32), "Cancel") {
                            self.action_window_position = None;
                            self.action_window_size = None;
                            self.client
                                .send(Message::CancelSelectAction {
                                    player_id: self.player_id,
                                    game_id: self.game_id,
                                })
                                .unwrap();
                        }
                    },
                );
            }
            Phase::SelectingCardOutsideRealm {
                player_id,
                owner,
                spellbook,
                cemetery,
                hand,
                after_select,
            } => {
                if player_id != &self.player_id {
                    return Ok(());
                }

                let mut number_of_zones = 0;
                if spellbook.is_some() {
                    number_of_zones += 1;
                }

                if hand.is_some() {
                    number_of_zones += 1;
                }

                if cemetery.is_some() {
                    number_of_zones += 1;
                }

                let width = screen_width() - 20.0;
                let height = screen_height() - 20.0;
                let mut images = HashMap::new();
                let cards_to_display: Vec<Card> = self
                    .cards
                    .iter()
                    .filter(|c| {
                        if owner.is_some() {
                            return c.card.get_owner_id() == owner.as_ref().unwrap();
                        }

                        true
                    })
                    .map(|c| c.card.clone())
                    .collect();

                for card in &cards_to_display {
                    images.insert(card.get_name(), TextureCache::get_card_texture(&card).await);
                }

                let mut all_valid_cards: Vec<uuid::Uuid> = spellbook.as_deref().unwrap_or_default().into();
                all_valid_cards.extend(cemetery.as_deref().unwrap_or_default());
                all_valid_cards.extend(hand.as_deref().unwrap_or_default());
                ui::root_ui().window(
                    CARD_SELECTION_WINDOW_ID,
                    Vec2::new(10.0, 10.0),
                    Vec2::new(width, height),
                    |ui| {
                        for (idx, card) in cards_to_display.iter().enumerate() {
                            if Texture::new(images.get(card.get_name()).unwrap().clone())
                                .position(Vec2::new(10.0, 30.0 * idx as f32))
                                .size(card_width(), card_height())
                                .ui(ui)
                            {
                                self.client
                                    .send(Message::SummonMinion {
                                        player_id: self.player_id.clone(),
                                        card_id: card.get_id().clone(),
                                        game_id: self.game_id.clone(),
                                        cell_id: 0,
                                    })
                                    .unwrap();
                            }

                            // if spellbook.as_deref().unwrap_or_default().contains(card.get_id()) {
                            //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
                            // }
                            //
                            // if cemetery.as_deref().unwrap_or_default().contains(card.get_id()) {
                            //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
                            // }
                            //
                            // if hand.as_deref().unwrap_or_default().contains(card.get_id()) {
                            //     draw_rectangle_lines(10.0, 30.0 * idx as f32, card_width(), card_height(), 3.0, GREEN);
                            // }
                        }
                    },
                );
            }
            Phase::SelectingCard {
                player_id, card_ids, ..
            } if player_id == &self.player_id => {
                let valid_cards: Vec<&CardDisplay> = self
                    .cards
                    .iter()
                    .filter(|c| card_ids.contains(&c.card.get_id()))
                    .collect();

                let has_selected_card = valid_cards.iter().any(|c| c.is_selected);
                let mut scale = 1.0;
                for card in valid_cards {
                    if card.is_hovered {
                        scale = 1.2;
                    }

                    if !has_selected_card || card.is_selected {
                        draw_rectangle_lines(
                            card.rect.x,
                            card.rect.y,
                            card.rect.w * scale,
                            card.rect.h * scale,
                            3.0,
                            WHITE,
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn render_card_preview(&self) -> anyhow::Result<()> {
        let selected_card = self.cards.iter().find(|card_display| card_display.is_hovered);
        let screen_rect = screen_rect();

        if let Some(card_display) = selected_card {
            const PREVIEW_SCALE: f32 = 2.7;
            let mut dimensions = spell_dimensions() * PREVIEW_SCALE;
            if card_display.card.is_site() {
                dimensions = site_dimensions() * PREVIEW_SCALE;
            }

            let preview_x = 20.0;
            let preview_y = screen_rect.h - dimensions.y - 20.0;
            draw_texture_ex(
                &TextureCache::get_card_texture(&card_display.card).await,
                preview_x,
                preview_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(dimensions),
                    ..Default::default()
                },
            );
        }

        Ok(())
    }

    async fn render_cemetery(&self) -> anyhow::Result<()> {
        let discarded_cards = self
            .cards
            .iter()
            .filter(|card_display| card_display.card.get_zone() == &CardZone::Cemetery)
            .collect::<Vec<&CardDisplay>>();
        for card in discarded_cards {
            let cemetery_rect = cemetery_rect();
            draw_texture_ex(
                &TextureCache::get_card_texture(&card.card).await,
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
        for (idx, card_display) in self.cards.iter().enumerate() {
            if card_display.rect.contains(mouse_position.into()) {
                hovered_card_index = Some(idx);
            };
        }

        for card in &mut self.cards {
            card.is_hovered = false;
        }

        if let Some(idx) = hovered_card_index {
            self.cards.get_mut(idx).unwrap().is_hovered = true;
        }

        match &self.state.phase {
            Phase::WaitingForPlay { player_id } if player_id == &self.player_id => {
                let mut card_selected = None;
                for card_display in &mut self
                    .cards
                    .iter_mut()
                    .filter(|c| matches!(c.card.get_zone(), CardZone::Realm(_)) || c.card.get_zone() == &CardZone::Hand)
                {
                    if card_display.is_hovered && is_mouse_button_released(MouseButton::Left) {
                        card_selected = Some(card_display.card.get_id().clone());
                        break;
                    };
                }

                if card_selected.is_some() {
                    self.client
                        .send(Message::SelectCard {
                            card_id: card_selected.unwrap(),
                            player_id: self.player_id,
                            game_id: self.game_id,
                        })
                        .unwrap();
                }
            }
            Phase::SelectingCard {
                player_id,
                card_ids,
                after_select,
                ..
            } if player_id == &self.player_id => {
                let valid_cards: Vec<&CardDisplay> = self
                    .cards
                    .iter()
                    .filter(|c| card_ids.contains(&c.card.get_id()))
                    .collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.card.get_id().clone());
                    }
                }

                if let Some(id) = selected_id {
                    let card = self.cards.iter_mut().find(|c| c.card.get_id() == &id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        match after_select {
                            Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets { card_id })) => {
                                self.client
                                    .send(Message::PlayCard {
                                        player_id: self.player_id,
                                        card_id: card_id.clone(),
                                        game_id: self.game_id,
                                        targets: Target::Card(id.clone()),
                                    })
                                    .unwrap();
                            }
                            Some(Action::GameAction(GameAction::PlaySelectedCard)) => {
                                self.client
                                    .send(Message::PrepareCardForPlay {
                                        player_id: self.player_id,
                                        card_id: card.card.get_id().clone(),
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                            Some(Action::GameAction(GameAction::AttackSelectedTarget { attacker_id })) => {
                                self.client
                                    .send(Message::AttackTarget {
                                        player_id: self.player_id,
                                        attacker_id: attacker_id.clone(),
                                        target_id: card.card.get_id().clone(),
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_cell_click(&mut self, mouse_position: Vec2) {
        if let Phase::SelectingCell {
            cell_ids,
            player_id,
            after_select,
        } = &self.state.phase
        {
            if player_id != &self.player_id {
                return;
            }

            let mut played_card = false;
            for (idx, cell) in self.cells.iter().enumerate() {
                if !cell_ids.contains(&cell.id) {
                    continue;
                }

                if cell.rect.contains(mouse_position.into()) {
                    let cell_id = self.cells[idx].id;
                    if is_mouse_button_released(MouseButton::Left) {
                        match after_select {
                            Some(Action::GameAction(GameAction::PlayCardOnSelectedTargets { card_id })) => {
                                self.client
                                    .send(Message::PlayCard {
                                        player_id: self.player_id,
                                        card_id: card_id.clone(),
                                        targets: Target::Cell(cell_id),
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                            Some(Action::GameAction(GameAction::MoveCardToSelectedCell { card_id })) => {
                                self.client
                                    .send(Message::MoveCard {
                                        player_id: self.player_id,
                                        card_id: card_id.clone(),
                                        cell_id,
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                            Some(Action::GameAction(GameAction::SummonMinionToSelectedCell { card_id })) => {
                                println!("Summoning minion to cell {}", cell_id);
                                self.client
                                    .send(Message::SummonMinion {
                                        player_id: self.player_id,
                                        card_id: card_id.clone(),
                                        cell_id: cell_id,
                                        game_id: self.game_id,
                                    })
                                    .unwrap();
                            }
                            _ => {}
                        }
                        played_card = true;
                    }
                }
            }

            if played_card {
                self.clear_selected_cards();
            }
        }
    }

    fn clear_selected_cards(&mut self) {
        for card_display in &mut self.cards {
            card_display.is_selected = false;
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

            if let Phase::SelectingCell {
                cell_ids, player_id, ..
            } = &self.state.phase
            {
                if &self.player_id != player_id {
                    continue;
                }

                if cell_ids.contains(&cell_display.id) {
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
        for card_display in &self.cards {
            if !matches!(card_display.card.get_zone(), CardZone::Realm(_)) {
                continue;
            }

            let img = TextureCache::get_card_texture(&card_display.card).await;
            let mut rotation = 0.0;
            if card_display.card.is_tapped() {
                rotation = std::f32::consts::FRAC_PI_2;
            }

            draw_texture_ex(
                &img,
                card_display.rect.x,
                card_display.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(card_display.rect.w, card_display.rect.h) * CARD_IN_PLAY_SCALE),
                    rotation,
                    ..Default::default()
                },
            );
        }
    }

    async fn render_player_hand(&self) {
        for card_display in &self.cards {
            if card_display.card.get_zone() != &CardZone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_display.is_selected || card_display.is_hovered {
                scale = 1.2;
            }

            let img = TextureCache::get_card_texture(&card_display.card).await;
            draw_texture_ex(
                &img,
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
            &TextureCache::get_texture(SPELLBOOK_IMAGE, "spellbook", false).await,
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
            &TextureCache::get_texture(ATLASBOOK_IMAGE, "atlas", false).await,
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
        match self.state.phase {
            Phase::WaitingForCardDraw {
                player_id, ref types, ..
            } if player_id == self.player_id && types.contains(&card_type) => {
                let message = networking::Message::DrawCard {
                    card_type,
                    player_id: self.player_id,
                    game_id: self.game_id,
                };
                self.client.send(message)
            }
            _ => Ok(()),
        }
    }

    async fn update_cards_in_realm(&mut self, cards: &[Card]) -> anyhow::Result<()> {
        for card in cards {
            if let CardZone::Realm(cell_id) = card.get_zone() {
                let cell_rect = self.cells.iter().find(|c| c.id == *cell_id).unwrap().rect;
                let mut dimensions = spell_dimensions();
                if card.is_site() {
                    dimensions = site_dimensions();
                }

                let rect = Rect::new(
                    cell_rect.x + (cell_rect.w - dimensions.x) / 2.0,
                    cell_rect.y + (cell_rect.h - dimensions.y) / 2.0,
                    dimensions.x,
                    dimensions.y,
                );

                self.cards.push(CardDisplay {
                    card: card.clone(),
                    rect,
                    is_hovered: false,
                    is_selected: false,
                    rotation: 0.0,
                });
            }
        }

        Ok(())
    }

    fn update_cards_in_hand(&mut self, cards: &[Card]) -> anyhow::Result<()> {
        let spells: Vec<&Card> = cards
            .iter()
            .filter(|c| c.get_zone() == &CardZone::Hand)
            .filter(|c| c.get_owner_id() == &self.player_id)
            .filter(|c| c.is_spell())
            .collect();

        let sites: Vec<&Card> = cards
            .iter()
            .filter(|c| c.get_zone() == &CardZone::Hand)
            .filter(|c| c.get_owner_id() == &self.player_id)
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

            let current_card = self.cards.iter().find(|c| c.card.get_id() == card.get_id());
            let mut is_hovered = false;
            if let Some(c) = current_card {
                is_hovered = c.is_hovered;
            }

            displays.push(CardDisplay {
                card: (*card).clone(),
                rect,
                is_hovered,
                is_selected: false,
                rotation: rad,
            });
        }

        let site_x = hand_rect.x + spell_hand_size as f32 * CARD_OFFSET_X + 40.0;
        for (idx, card) in sites.iter().enumerate() {
            let dimensions = site_dimensions();
            let x = site_x;
            let y = hand_rect.y + 20.0 + 20.0 * idx as f32;

            let rect = Rect::new(x, y, dimensions.x, dimensions.y);

            let current_card = self.cards.iter().find(|c| c.card.get_id() == card.get_id());
            let mut is_hovered = false;
            if let Some(c) = current_card {
                is_hovered = c.is_hovered;
            }

            displays.push(CardDisplay {
                card: (*card).clone(),
                rect,
                is_hovered,
                is_selected: false,
                rotation: 0.0,
            });
        }

        self.cards = displays;
        Ok(())
    }
}
