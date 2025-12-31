use crate::{
    components::Component,
    config::{site_dimensions, spell_dimensions},
    render::CardRect,
    scene::game::Status,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, DARKGREEN, WHITE},
    math::{Rect, Vec2},
    shapes::{DrawRectangleParams, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines},
    texture::{DrawTextureParams, draw_texture_ex},
};
use sorcerers::card::{RenderableCard, Zone};

#[derive(Debug)]
pub struct PlayerHandComponent {
    pub player_id: uuid::Uuid,
    pub rect: Rect,
    pub rects: Vec<CardRect>,
    pub status: Status,
}

impl PlayerHandComponent {
    pub fn new(rect: Rect, player_id: &uuid::Uuid) -> Self {
        Self {
            rect,
            player_id: player_id.clone(),
            rects: Vec::new(),
            status: Status::Idle,
        }
    }

    async fn compute_rects(&mut self, cards: &[RenderableCard]) -> anyhow::Result<()> {
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
        let start_x = self.rect.x + (self.rect.w - total_width) / 2.0;
        let spells_y = self.rect.y + self.rect.h / 2.0 - spell_dim.y / 2.0;

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
            let sites_start_y = self.rect.y + self.rect.h / 2.0 - spell_dim.y / 2.0;
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
    async fn update(&mut self, cards: &[RenderableCard], status: Status) -> anyhow::Result<()> {
        self.status = status;
        self.compute_rects(cards).await
    }

    async fn render(&mut self) {
        let bg_color = Color::new(0.15, 0.18, 0.22, 0.85);
        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg_color);

        for card_rect in &self.rects {
            if card_rect.zone != Zone::Hand {
                continue;
            }

            let mut scale = 1.0;
            if card_rect.is_selected || card_rect.is_hovered {
                if let Status::SelectingCard { preview: false, .. } = &self.status {
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
            } = &self.status
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
    }
}
