use macroquad::{
    color::{Color, BLUE, GREEN, RED, WHITE},
    input::{is_mouse_button_released, mouse_position, MouseButton},
    math::{Rect, Vec2},
    shapes::{draw_line, draw_rectangle_lines, draw_triangle_lines},
    text::draw_text,
    texture::{draw_texture_ex, DrawTextureParams},
    ui,
};
use sorcerers::{
    card::{Card, CardType, CardZone},
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

#[derive(Debug)]
pub struct Game {
    pub player_id: uuid::Uuid,
    pub game_id: uuid::Uuid,
    pub cards: Vec<CardDisplay>,
    pub cells: Vec<CellDisplay>,
    pub client: networking::client::Client,
    pub state: State,
}

impl Game {
    pub fn new(player_id: uuid::Uuid, client: networking::client::Client) -> Self {
        let cells = (0..20)
            .map(|i| {
                let col = i % 5;
                let row = i / 5;
                let rect = Rect::new(
                    REALM_RECT.x + col as f32 * (REALM_RECT.w / 5.0),
                    REALM_RECT.y + row as f32 * (REALM_RECT.h / 4.0),
                    REALM_RECT.w / 5.0,
                    REALM_RECT.h / 4.0,
                );
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
        }
    }

    pub async fn update(&mut self) {
        let mouse_position = mouse_position().into();
        self.handle_card_selection(mouse_position);
        self.handle_cell_selection(mouse_position);
    }

    pub async fn render(&mut self) -> anyhow::Result<()> {
        self.render_background().await;
        self.render_grid().await;
        self.render_deck().await;
        self.render_player_hand().await;
        self.render_realm().await;
        self.render_discard_pile().await?;
        self.render_gui().await?;
        self.render_card_preview().await?;
        Ok(())
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        if is_mouse_button_released(MouseButton::Left) {
            let mouse_position = mouse_position();
            if ATLASBOOK_RECT.contains(mouse_position.into()) {
                self.draw_card(CardType::Site).unwrap();
            }

            if SPELLBOOK_RECT.contains(mouse_position.into()) {
                self.draw_card(CardType::Spell).unwrap();
            }
        }

        None
    }

    pub async fn process_message(&mut self, message: networking::Message) -> anyhow::Result<()> {
        match message {
            Message::MatchCreated { game_id, .. } => {
                if !self.state.is_player_one(&self.player_id) {
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
                self.update_cards_in_hand(&state.cards)?;
                self.update_cards_in_realm(&state.cards).await?;
                Ok(())
            }
            _ => Ok(()),
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
        let mana_text = format!("Mana: {}", resources.mana);
        draw_text(&mana_text, x, y, FONT_SIZE, WHITE);

        let thresholds_y: f32 = y + 10.0;
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

        const BASE_X: f32 = 20.0;
        const PLAYER_Y: f32 = SCREEN_HEIGHT - 40.0;
        let resources = self
            .state
            .resources
            .get(&self.player_id)
            .cloned()
            .unwrap_or(Resources::new());
        Game::render_resources(BASE_X, PLAYER_Y, &resources);

        const OPPONENT_Y: f32 = 25.0;
        let opponent_resources = self
            .state
            .resources
            .iter()
            .find(|(player_id, _)| **player_id != self.player_id)
            .map(|(_, resources)| resources.clone())
            .unwrap_or(Resources::new());
        Game::render_resources(BASE_X, OPPONENT_Y, &opponent_resources);

        let turn_label = if self.state.is_players_turn(&self.player_id) {
            "Your Turn"
        } else {
            "Opponent's Turn"
        };

        draw_text(turn_label, SCREEN_WIDTH / 2.0 - 50.0, 30.0, FONT_SIZE, WHITE);

        let is_in_turn = self.state.current_player == self.player_id;
        if is_in_turn {
            if ui::root_ui().button(Vec2::new(SCREEN_WIDTH - 100.0, SCREEN_HEIGHT - 40.0), "End Turn") {
                self.client.send(Message::EndTurn {
                    player_id: self.player_id.clone(),
                    game_id: self.game_id.clone(),
                })?;
            }
        }

        Ok(())
    }

    async fn render_card_preview(&self) -> anyhow::Result<()> {
        let selected_card = self.cards.iter().find(|card_display| card_display.is_hovered);

        if let Some(card_display) = selected_card {
            const PREVIEW_SCALE: f32 = 2.7;
            let mut dimensions = SPELL_DIMENSIONS * PREVIEW_SCALE;
            if card_display.card.is_site() {
                dimensions = SITE_DIMENSIONS * PREVIEW_SCALE;
            }

            let preview_x = 20.0;
            let preview_y = SCREEN_HEIGHT - dimensions.y - 20.0;
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

    async fn render_discard_pile(&self) -> anyhow::Result<()> {
        let discarded_cards = self
            .cards
            .iter()
            .filter(|card_display| card_display.card.get_zone() == &CardZone::DiscardPile)
            .collect::<Vec<&CardDisplay>>();
        for card in discarded_cards {
            draw_texture_ex(
                &TextureCache::get_card_texture(&card.card).await,
                DISCARD_PILE_RECT.x,
                DISCARD_PILE_RECT.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(DISCARD_PILE_RECT.size() * CARD_IN_PLAY_SCALE),
                    ..Default::default()
                },
            );
        }

        Ok(())
    }

    fn cards_in_hand_mut(&mut self) -> Vec<&mut CardDisplay> {
        self.cards
            .iter_mut()
            .filter(|card_display| card_display.card.get_zone() == &CardZone::Hand)
            .collect()
    }

    fn handle_card_selection(&mut self, mouse_position: Vec2) {
        if let Phase::WaitingForPlay { player_id } = self.state.phase {
            let mut hovered_card_index = None;
            for (idx, card_display) in self.cards.iter().enumerate() {
                if card_display.card.get_zone() != &CardZone::Hand {
                    continue;
                }

                if card_display.rect.contains(mouse_position.into()) {
                    hovered_card_index = Some(idx);
                };
            }

            for card in &mut self.cards_in_hand_mut() {
                card.is_hovered = false;
            }

            if let Some(idx) = hovered_card_index {
                self.cards_in_hand_mut().get_mut(idx).unwrap().is_hovered = true;
            }

            if player_id != self.player_id {
                return;
            }

            let mut card_selected = None;
            for card_display in self.cards_in_hand_mut() {
                if card_display.card.get_zone() != &CardZone::Hand {
                    continue;
                }

                if card_display.is_hovered && is_mouse_button_released(MouseButton::Left) {
                    card_display.is_selected = !card_display.is_selected;
                    if card_display.is_selected {
                        card_selected = Some(card_display.card.get_id().clone());
                    }
                };
            }

            if card_selected.is_some() {
                self.client
                    .send(Message::CardSelected {
                        card_id: card_selected.unwrap(),
                        player_id: self.player_id,
                        game_id: self.game_id,
                    })
                    .unwrap();
            }
        }
    }

    fn get_selected_card_id(&self) -> Option<&uuid::Uuid> {
        for card_display in &self.cards {
            if card_display.is_selected {
                return Some(card_display.card.get_id());
            }
        }
        None
    }

    fn handle_cell_selection(&mut self, mouse_position: Vec2) {
        if let Phase::SelectingCell { cell_ids, player_id } = &self.state.phase {
            if player_id != &self.player_id {
                return;
            }

            for (idx, cell) in self.cells.iter().enumerate() {
                if !cell_ids.contains(&cell.id) {
                    continue;
                }

                if cell.rect.contains(mouse_position.into()) {
                    let cell_id = self.cells[idx].id;
                    if is_mouse_button_released(MouseButton::Left) {
                        self.client
                            .send(Message::CardPlayed {
                                player_id: self.player_id,
                                card_id: self.get_selected_card_id().cloned().unwrap(),
                                cell_id,
                                game_id: self.game_id,
                            })
                            .unwrap();
                    }
                }
            }
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

            if let Phase::SelectingCell { cell_ids, player_id } = &self.state.phase {
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
            draw_texture_ex(
                &img,
                card_display.rect.x,
                card_display.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(card_display.rect.w, card_display.rect.h) * CARD_IN_PLAY_SCALE),
                    // rotation,
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

            if card_display.is_selected {
                draw_rectangle_lines(
                    card_display.rect.x,
                    card_display.rect.y,
                    card_display.rect.w * scale,
                    card_display.rect.h * scale,
                    3.0,
                    WHITE,
                );
            }
        }
    }

    async fn render_background(&self) {
        draw_texture_ex(
            &TextureCache::get_texture(REALM_BACKGROUND_IMAGE, false).await,
            REALM_RECT.x,
            REALM_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(REALM_RECT.w, REALM_RECT.h)),
                ..Default::default()
            },
        );
    }

    async fn render_deck(&self) {
        draw_texture_ex(
            &TextureCache::get_texture(SPELLBOOK_IMAGE, false).await,
            SPELLBOOK_RECT.x,
            SPELLBOOK_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(SPELLBOOK_RECT.size() * CARD_IN_PLAY_SCALE),
                ..Default::default()
            },
        );

        draw_texture_ex(
            &TextureCache::get_texture(ATLASBOOK_IMAGE, false).await,
            ATLASBOOK_RECT.x,
            ATLASBOOK_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(ATLASBOOK_RECT.size() * CARD_IN_PLAY_SCALE),
                ..Default::default()
            },
        );
    }

    fn draw_card(&self, card_type: CardType) -> anyhow::Result<()> {
        match self.state.phase {
            Phase::WaitingForCardDraw { player_id, count } if player_id == self.player_id => {
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
                let mut dimensions = SPELL_DIMENSIONS;
                if card.is_site() {
                    dimensions = SITE_DIMENSIONS;
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
        for (idx, card) in spells.iter().enumerate() {
            let dimensions = SPELL_DIMENSIONS;
            let angle = if spell_hand_size > 1 {
                ((idx as f32 - center) / center) * (fan_angle / 2.0)
            } else {
                0.0
            };
            let rad = angle.to_radians();
            let x = HAND_RECT.x + idx as f32 * CARD_OFFSET_X + 20.0;
            let y = HAND_RECT.y + 20.0 + radius * rad.sin();

            let rect = Rect::new(x, y, dimensions.x, dimensions.y);

            let current_card = self.cards.iter().find(|c| c.card.get_id() == card.get_id());
            let mut is_selected = false;
            let mut is_hovered = false;
            if let Some(c) = current_card {
                is_selected = c.is_selected;
                is_hovered = c.is_hovered;
            }

            displays.push(CardDisplay {
                card: (*card).clone(),
                rect,
                is_hovered,
                is_selected,
                rotation: rad,
            });
        }

        let site_x = HAND_RECT.x + spell_hand_size as f32 * CARD_OFFSET_X + 40.0;
        for (idx, card) in sites.iter().enumerate() {
            let dimensions = SITE_DIMENSIONS;
            let x = site_x;
            let y = HAND_RECT.y + 20.0 + 20.0 * idx as f32;

            let rect = Rect::new(x, y, dimensions.x, dimensions.y);

            let current_card = self.cards.iter().find(|c| c.card.get_id() == card.get_id());
            let mut is_selected = false;
            let mut is_hovered = false;
            if let Some(c) = current_card {
                is_selected = c.is_selected;
                is_hovered = c.is_hovered;
            }

            displays.push(CardDisplay {
                card: (*card).clone(),
                rect,
                is_hovered,
                is_selected,
                rotation: 0.0,
            });
        }

        self.cards = displays;
        Ok(())
    }
}
