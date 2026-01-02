use crate::{
    clicks_enabled,
    components::{Component, ComponentAction},
    config::{hand_rect, site_dimensions, spell_dimensions},
    render::CardRect,
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, DARKGREEN, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{Rect, Vec2},
    shapes::{DrawRectangleParams, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines},
    texture::{DrawTextureParams, draw_texture_ex},
};
use sorcerers::{
    card::{RenderableCard, Zone},
    networking::{self, message::ClientMessage},
};

#[derive(Debug)]
pub struct PlayerHandComponent {
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
    rects: Vec<CardRect>,
    client: networking::client::Client,
    visible: bool,
}

impl PlayerHandComponent {
    pub fn new(game_id: &uuid::Uuid, player_id: &uuid::Uuid, client: networking::client::Client) -> Self {
        Self {
            game_id: game_id.clone(),
            player_id: player_id.clone(),
            rects: Vec::new(),
            client,
            visible: true,
        }
    }

    async fn compute_rects(&mut self, cards: &[RenderableCard]) -> anyhow::Result<()> {
        let rect = hand_rect();
        let spell_count = cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_spell())
            .count();

        let site_count = cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_site())
            .count();

        let mut displays: Vec<CardRect> = Vec::new();

        // Layout parameters
        let spell_dim = spell_dimensions();
        let site_dim = site_dimensions();
        let card_spacing = 20.0;

        // Calculate total width for spells (horizontal row)
        let spells_width = if spell_count > 0 {
            spell_count as f32 * spell_dim.x + (spell_count as f32 - 1.0) * card_spacing
        } else {
            0.0
        };

        // Combined width for centering: spells row + spacing + site card width (if any sites)
        let total_width = spells_width + if site_count > 0 { card_spacing + site_dim.x } else { 0.0 };

        // Center horizontally in hand area
        let start_x = rect.x + (rect.w - total_width) / 2.0;
        let spells_y = rect.y + rect.h / 2.0 - spell_dim.y / 2.0;

        // Spells row
        for (idx, card) in cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_spell())
            .enumerate()
        {
            let x = start_x + idx as f32 * (spell_dim.x + card_spacing);
            let rect = Rect::new(x, spells_y, spell_dim.x, spell_dim.y);

            displays.push(CardRect {
                id: card.id,
                owner_id: card.owner_id,
                rect,
                is_hovered: false,
                is_selected: false,
                zone: card.zone.clone(),
                tapped: card.tapped,
                image: TextureCache::get_card_texture(card).await,
                modifiers: card.modifiers.clone(),
                damage_taken: card.damage_taken.clone(),
            });
        }

        // Sites column, stacked vertically to the right of spells
        if site_count > 0 {
            let sites_x = start_x + spells_width + card_spacing;
            let sites_start_y = rect.y + rect.h / 2.0 - spell_dim.y / 2.0;
            for (idx, card) in cards
                .iter()
                .filter(|c| c.zone == Zone::Hand)
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.is_site())
                .enumerate()
            {
                let y = sites_start_y + idx as f32 * 20.0;
                let rect = Rect::new(sites_x, y, site_dim.x, site_dim.y);

                displays.push(CardRect {
                    id: card.id,
                    owner_id: card.owner_id,
                    rect,
                    is_hovered: false,
                    is_selected: false,
                    zone: card.zone.clone(),
                    tapped: card.tapped,
                    image: TextureCache::get_card_texture(card).await,
                    modifiers: card.modifiers.clone(),
                    damage_taken: 0,
                });
            }
        }

        self.rects = displays;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Component for PlayerHandComponent {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        self.compute_rects(&data.cards).await
    }

    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        let rect = hand_rect();
        let bg_color = Color::new(0.15, 0.18, 0.22, 0.85);
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg_color);

        for card_rect in &self.rects {
            if card_rect.zone != Zone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_rect.is_selected || card_rect.is_hovered {
                if let Status::SelectingCard { preview: false, .. } = data.status {
                    scale = 1.2;
                }
            }

            let rect = card_rect.rect;
            draw_texture_ex(
                &card_rect.image,
                rect.x,
                rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(rect.w, rect.h) * scale),
                    rotation: card_rect.rotation().clone(),
                    ..Default::default()
                },
            );

            draw_rectangle_lines(rect.x, rect.y, rect.w * scale, rect.h * scale, 5.0, DARKGREEN);

            if let Status::SelectingCard {
                cards, preview: false, ..
            } = &data.status
            {
                if !cards.contains(&card_rect.id) {
                    draw_rectangle_ex(
                        rect.x,
                        rect.y,
                        rect.w * scale,
                        rect.h * scale,
                        DrawRectangleParams {
                            color: Color::new(200.0, 200.0, 200.0, 0.6),
                            rotation: card_rect.rotation(),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        Ok(())
    }

    fn process_input(&mut self, in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentAction>> {
        let mouse_position = macroquad::input::mouse_position();
        if !clicks_enabled() {
            return Ok(None);
        }

        if let Status::SelectingAction { .. } = &data.status {
            return Ok(None);
        }

        if !in_turn {
            return Ok(None);
        }

        let mut hovered_card_index = None;
        for (idx, card_display) in self.rects.iter().enumerate() {
            if card_display.rect.contains(mouse_position.into()) {
                hovered_card_index = Some(idx);
            };
        }

        for card in &mut self.rects {
            card.is_hovered = false;
        }

        if let Some(idx) = hovered_card_index {
            self.rects.get_mut(idx).unwrap().is_hovered = true;
        }

        match &data.status {
            Status::Idle => {
                for card_rect in &mut self
                    .rects
                    .iter_mut()
                    .filter(|c| c.zone.is_in_realm() || c.zone == Zone::Hand)
                {
                    if card_rect.is_hovered && is_mouse_button_released(MouseButton::Left) {
                        self.client
                            .send(ClientMessage::ClickCard {
                                card_id: card_rect.id.clone(),
                                player_id: self.player_id,
                                game_id: self.game_id,
                            })
                            .unwrap();
                    };
                }
            }
            Status::SelectingCard {
                cards, preview: true, ..
            } => {
                let valid_cards: Vec<&CardRect> = self.rects.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
                    let card = self.rects.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        data.status = Status::Idle;
                    }
                }
            }

            Status::SelectingCard {
                cards, preview: false, ..
            } => {
                let valid_cards: Vec<&CardRect> = self.rects.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
                    let card = self.rects.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        data.status = Status::Idle;
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}
