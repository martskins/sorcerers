use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    input::Mouse,
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, DARKGREEN, WHITE},
    input::{MouseButton, is_mouse_button_released, mouse_position},
    math::{Rect, Vec2},
    shapes::{DrawRectangleParams, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines},
    texture::{DrawTextureParams, draw_texture_ex},
};
use sorcerers::{
    card::{CardData, Zone},
    networking::{self, message::ClientMessage},
};

#[derive(Debug)]
pub struct PlayerHandComponent {
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
    cards: Vec<CardRect>,
    client: networking::client::Client,
    visible: bool,
    rect: Rect,
    last_mouse_pos: Vec2,
}

impl PlayerHandComponent {
    pub fn new(game_id: &uuid::Uuid, player_id: &uuid::Uuid, client: networking::client::Client, rect: Rect) -> Self {
        Self {
            game_id: game_id.clone(),
            player_id: player_id.clone(),
            cards: Vec::new(),
            client,
            visible: true,
            rect,
            last_mouse_pos: mouse_position().into(),
        }
    }

    fn card_width(&self) -> f32 {
        self.card_height() * CARD_ASPECT_RATIO
    }

    fn card_height(&self) -> f32 {
        self.rect.h * 0.8
    }

    fn spell_dimensions(&self) -> Vec2 {
        Vec2::new(self.card_width(), self.card_height())
    }

    pub fn site_dimensions(&self) -> Vec2 {
        Vec2::new(self.card_height(), self.card_width())
    }

    async fn compute_rects(&mut self, cards: &[CardData]) -> anyhow::Result<()> {
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

        let spell_dim = self.spell_dimensions();
        let site_dim = self.site_dimensions();
        let card_spacing = 20.0;

        let spells_width = if spell_count > 0 {
            spell_count as f32 * spell_dim.x + (spell_count as f32 - 1.0) * card_spacing
        } else {
            0.0
        };

        let total_width = spells_width + if site_count > 0 { card_spacing + site_dim.x } else { 0.0 };
        let start_x = self.rect.x + (self.rect.w - total_width) / 2.0;
        let spells_y = self.rect.y + self.rect.h / 2.0 - spell_dim.y / 2.0;

        let mut rects: Vec<CardRect> = Vec::new();
        for (idx, card) in cards
            .iter()
            .filter(|c| c.zone == Zone::Hand)
            .filter(|c| c.owner_id == self.player_id)
            .filter(|c| c.is_spell())
            .enumerate()
        {
            if let Some(card) = self.cards.iter().find(|c| c.id == card.id) {
                rects.push(card.clone());
                continue;
            }

            let x = start_x + idx as f32 * (spell_dim.x + card_spacing);
            let rect = Rect::new(x, spells_y, spell_dim.x, spell_dim.y);

            rects.push(CardRect {
                id: card.id,
                owner_id: card.owner_id,
                rect,
                is_hovered: self
                    .cards
                    .iter()
                    .find(|r| r.id == card.id)
                    .map_or(false, |r| r.is_hovered),
                zone: card.zone.clone(),
                tapped: card.tapped,
                image: TextureCache::get_card_texture(card).await,
                modifiers: card.modifiers.clone(),
                damage_taken: card.damage_taken.clone(),
                card_type: card.card_type.clone(),
                attached_to: card.attached_to.clone(),
            });
        }

        if site_count > 0 {
            let sites_x = start_x + spells_width + card_spacing;
            let sites_start_y = self.rect.y + self.rect.h / 2.0 - spell_dim.y / 2.0;
            for (idx, card) in cards
                .iter()
                .filter(|c| c.zone == Zone::Hand)
                .filter(|c| c.owner_id == self.player_id)
                .filter(|c| c.is_site())
                .enumerate()
            {
                if let Some(card) = self.cards.iter().find(|c| c.id == card.id) {
                    rects.push(card.clone());
                    continue;
                }

                let y = sites_start_y + idx as f32 * 20.0;
                let rect = Rect::new(sites_x, y, site_dim.x, site_dim.y);

                rects.push(CardRect {
                    id: card.id,
                    owner_id: card.owner_id,
                    rect,
                    is_hovered: self
                        .cards
                        .iter()
                        .find(|r| r.id == card.id)
                        .map_or(false, |r| r.is_hovered),
                    zone: card.zone.clone(),
                    tapped: card.tapped,
                    image: TextureCache::get_card_texture(card).await,
                    modifiers: card.modifiers.clone(),
                    damage_taken: 0,
                    card_type: card.card_type.clone(),
                    attached_to: card.attached_to.clone(),
                });
            }
        }

        self.cards = rects;
        Ok(())
    }

    async fn render_card_preview(&self, data: &mut GameData) -> anyhow::Result<()> {
        if let Some(card) = self.cards.iter().find(|card| card.is_hovered) {
            render::render_card_preview(card, data).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Component for PlayerHandComponent {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        let new_mouse_pos: Vec2 = mouse_position().into();
        let mouse_delta = new_mouse_pos - self.last_mouse_pos;

        self.compute_rects(&data.cards).await?;

        let mut dragging_card: Option<uuid::Uuid> = None;
        for card in &mut self.cards {
            if card.is_hovered && Mouse::dragging().await {
                dragging_card = Some(card.id.clone());
            }
        }

        if let Some(card_id) = dragging_card {
            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                if card.zone == Zone::Hand {
                    let min_x = self.rect.x;
                    let max_x = self.rect.x + self.rect.w - card.rect.w;
                    let min_y = self.rect.y;
                    let max_y = self.rect.y + self.rect.h - card.rect.h;
                    card.rect.x = (card.rect.x + mouse_delta.x).clamp(min_x, max_x);
                    card.rect.y = (card.rect.y + mouse_delta.y).clamp(min_y, max_y);
                }
            }
        }

        self.last_mouse_pos = new_mouse_pos;
        Ok(())
    }

    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        let bg_color = Color::new(0.15, 0.18, 0.22, 0.85);
        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg_color);

        for card_rect in &self.cards {
            if card_rect.zone != Zone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_rect.is_hovered {
                scale = 1.1;
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

        self.render_card_preview(data).await?;

        Ok(())
    }

    async fn process_input(&mut self, in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentCommand>> {
        let mouse_position = macroquad::input::mouse_position();
        if !Mouse::enabled().await {
            return Ok(None);
        }

        if let Status::SelectingAction { .. } = &data.status {
            return Ok(None);
        }

        if !in_turn {
            return Ok(None);
        }

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

        match &data.status {
            Status::Idle => {
                for card_rect in &mut self
                    .cards
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
                let valid_cards: Vec<&CardRect> = self.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
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

            Status::SelectingCard {
                cards, preview: false, ..
            } => {
                let valid_cards: Vec<&CardRect> = self.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && is_mouse_button_released(MouseButton::Left) {
                        selected_id = Some(card.id.clone());
                    }
                }

                if let Some(id) = selected_id {
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
            _ => {}
        }

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    async fn process_command(&mut self, command: &ComponentCommand) {
        match command {
            ComponentCommand::SetRect {
                component_type: ComponentType::PlayerHand,
                rect,
            } => self.rect = rect.clone(),
            _ => {}
        }
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::PlayerHand
    }
}
