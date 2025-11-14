use macroquad::{
    color::WHITE,
    input::{is_mouse_button_released, mouse_position, MouseButton},
    math::{Rect, Vec2},
    shapes::draw_rectangle_lines,
    text::draw_text,
    texture::{draw_texture_ex, DrawTextureParams},
};
use sorcerers::{
    card::{Card, CardType, CardZone},
    networking::{self, Message},
};

use crate::{
    config::*,
    render::{CardDisplay, CellDisplay},
    scene::Scene,
    texture_cache::TextureCache,
};

#[derive(Debug, PartialEq)]
pub enum Status {
    InProgress,
    WaitingForCellSelection,
}

#[derive(Debug)]
pub struct Game {
    pub player_id: uuid::Uuid,
    pub cards: Vec<CardDisplay>,
    pub cells: Vec<CellDisplay>,
    pub status: Status,
    pub client: networking::client::Client,
    pub is_player_one: bool,
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
                CellDisplay {
                    id: i as u8 + 1,
                    rect,
                }
            })
            .collect();
        Self {
            player_id,
            cards: vec![],
            cells,
            status: Status::InProgress,
            client,
            is_player_one: false,
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
        Ok(())
    }

    pub async fn process_input(&mut self) -> Option<Scene> {
        if is_mouse_button_released(MouseButton::Left) {
            // if !self.is_current_player() {
            //     return;
            // }

            let mouse_position = mouse_position();
            // if self.state.phase == Phase::WaitingForCardDrawPhase {
            // let current_player = self.players.get_mut(&self.player_id).unwrap();
            // let mut drew_card = false;
            if ATLASBOOK_RECT.contains(mouse_position.into()) {
                self.draw_card(CardType::Site).unwrap();
                // current_player.draw_site();
                // drew_card = true;
            }

            if SPELLBOOK_RECT.contains(mouse_position.into()) {
                self.draw_card(CardType::Spell).unwrap();
                // current_player.draw_spell();
                // drew_card = true;
            }

            // if drew_card {
            //     self.state.next_phase();
            // }
            // }
        }

        None
    }

    pub async fn process_message(&mut self, message: networking::Message) -> anyhow::Result<()> {
        match message {
            Message::MatchCreated { player1, .. } => {
                self.is_player_one = dbg!(self.player_id == player1);
                if !self.is_player_one {
                    for cell in &mut self.cells {
                        let new_id: i8 = cell.id as i8 - 21;
                        cell.id = new_id.abs() as u8;
                    }
                }

                Ok(())
            }
            Message::Sync { cards } => {
                self.update_cards_in_hand(&cards)?;
                self.update_cards_in_realm(&cards).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn render_discard_pile(&self) -> anyhow::Result<()> {
        let discarded_cards = self
            .cards
            .iter()
            .filter(|card_display| card_display.card.get_zone() == &CardZone::DiscardPile)
            .collect::<Vec<&CardDisplay>>();
        for card in discarded_cards {
            draw_texture_ex(
                &TextureCache::get_texture(&card.card.get_image()).await,
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

        for card_display in self.cards_in_hand_mut() {
            if card_display.card.get_zone() != &CardZone::Hand {
                continue;
            }

            if card_display.is_hovered && is_mouse_button_released(MouseButton::Left) {
                card_display.is_selected = !card_display.is_selected;
            };
        }

        let has_selected_card = self
            .cards_in_hand_mut()
            .iter()
            .any(|card_display| card_display.is_selected);
        self.status = if has_selected_card {
            Status::WaitingForCellSelection
        } else {
            Status::InProgress
        };
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
        if self.status != Status::WaitingForCellSelection {
            return;
        }

        let mut selected_cell_index = None;
        for (idx, cell) in self.cells.iter().enumerate() {
            if cell.rect.contains(mouse_position.into()) {
                selected_cell_index = Some(idx);
            }
        }

        if let Some(cell_idx) = selected_cell_index {
            let cell_id = self.cells[cell_idx].id;
            if is_mouse_button_released(MouseButton::Left) {
                self.status = Status::InProgress;
                self.client
                    .send(Message::CardPlayed {
                        player_id: self.player_id,
                        cell_id: cell_id as u8,
                        card_id: self.get_selected_card_id().cloned().unwrap(),
                    })
                    .unwrap();
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
        }
    }

    async fn render_realm(&self) {
        for card_display in &self.cards {
            if !matches!(card_display.card.get_zone(), CardZone::Realm(_)) {
                continue;
            }

            let img = TextureCache::get_texture(&card_display.card.get_image()).await;
            draw_texture_ex(
                &img,
                card_display.rect.x,
                card_display.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(
                        Vec2::new(card_display.rect.w, card_display.rect.h) * CARD_IN_PLAY_SCALE,
                    ),
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

            let img = TextureCache::get_texture(&card_display.card.get_image()).await;
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
            &TextureCache::get_texture(REALM_BACKGROUND_IMAGE).await,
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
            &TextureCache::get_texture(SPELLBOOK_IMAGE).await,
            SPELLBOOK_RECT.x,
            SPELLBOOK_RECT.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(SPELLBOOK_RECT.size() * CARD_IN_PLAY_SCALE),
                ..Default::default()
            },
        );

        draw_texture_ex(
            &TextureCache::get_texture(ATLASBOOK_IMAGE).await,
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
        let message = networking::Message::DrawCard {
            card_type,
            player_id: self.player_id,
        };
        self.client.send(message)
    }

    async fn update_cards_in_realm(&mut self, cards: &[Card]) -> anyhow::Result<()> {
        for card in cards {
            if let CardZone::Realm(cell_id) = card.get_zone() {
                let cell_rect = self.cells.iter().find(|c| c.id == *cell_id).unwrap().rect;
                let mut dimensions = SPELL_DIMENSIONS;
                if card.get_card_type() == sorcerers::card::CardType::Site {
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
            .filter(|c| c.get_card_type() == sorcerers::card::CardType::Spell)
            .collect();

        let sites: Vec<&Card> = cards
            .iter()
            .filter(|c| c.get_zone() == &CardZone::Hand)
            .filter(|c| c.get_owner_id() == &self.player_id)
            .filter(|c| c.get_card_type() == sorcerers::card::CardType::Site)
            .collect();

        let spell_hand_size = spells.len();
        let fan_angle = 30.0;
        let center = spell_hand_size as f32 / 2.0;
        let radius = 40.0;

        let mut displays: Vec<CardDisplay> = Vec::new();

        // Fanned spells
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

        // Stacked sites to the right of spells
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
