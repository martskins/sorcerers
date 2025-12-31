use crate::{
    clicks_enabled,
    components::Component,
    config::{CARD_IN_PLAY_SCALE, cell_rect, intersection_rect, site_dimensions, spell_dimensions},
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::game::Status,
    set_clicks_enabled,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, GRAY, GREEN, WHITE},
    math::{Rect, Vec2},
    shapes::{DrawRectangleParams, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines},
    text::draw_text,
    ui,
};
use rand::SeedableRng;
use sorcerers::card::{CardType, RenderableCard, Zone};

#[derive(Debug)]
pub struct RealmComponent {
    pub player_id: uuid::Uuid,
    pub rect: Rect,
    pub cell_rects: Vec<CellRect>,
    pub intersection_rects: Vec<IntersectionRect>,
    pub cards: Vec<CardRect>,
    pub status: Status,
}

impl RealmComponent {
    pub fn new(rect: Rect, player_id: &uuid::Uuid, mirror: bool) -> Self {
        let cell_rects: Vec<CellRect> = (0..20)
            .map(|i| {
                let rect = cell_rect(i + 1, mirror);
                CellRect { id: i as u8 + 1, rect }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    let rect = intersection_rect(&locs, mirror).unwrap();
                    Some(IntersectionRect { locations: locs, rect })
                }
                _ => None,
            })
            .collect();

        Self {
            rect,
            player_id: player_id.clone(),
            cards: Vec::new(),
            cell_rects,
            intersection_rects,
            status: Status::Idle,
        }
    }

    async fn compute_rects(&mut self, cards: &[RenderableCard]) -> anyhow::Result<()> {
        self.cards.clear();

        use rand::Rng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(1);
        for card in cards {
            match &card.zone {
                Zone::Realm(square) => {
                    let cell_rect = self.cell_rects.iter().find(|c| &c.id == square).unwrap().rect;
                    let mut dimensions = spell_dimensions();
                    if card.card_type == CardType::Site {
                        dimensions = site_dimensions();
                    }

                    let mut rect = Rect::new(
                        cell_rect.x + (cell_rect.w - dimensions.x) / 2.0,
                        cell_rect.y + (cell_rect.h - dimensions.y) / 2.0,
                        dimensions.x,
                        dimensions.y,
                    );

                    // Add jitter to position
                    // let mut rng = thread_rng();
                    let jitter_x: f32 = rng.random_range(-12.0..12.0);
                    let jitter_y: f32 = rng.random_range(-12.0..12.0);
                    rect.x += jitter_x;
                    rect.y += jitter_y;

                    self.cards.push(CardRect {
                        id: card.id,
                        owner_id: card.owner_id,
                        zone: card.zone.clone(),
                        tapped: card.tapped,
                        image: TextureCache::get_card_texture(&card).await,
                        rect,
                        is_hovered: false,
                        is_selected: false,
                        modifiers: card.modifiers.clone(),
                        damage_taken: card.damage_taken,
                    });
                }
                Zone::Intersection(locs) => {
                    let rect = self
                        .intersection_rects
                        .iter()
                        .find(|c| &c.locations == locs)
                        .unwrap()
                        .rect;
                    let mut dimensions = spell_dimensions();
                    if card.card_type == CardType::Site {
                        dimensions = site_dimensions();
                    }

                    let mut rect = Rect::new(rect.x, rect.y, dimensions.x, dimensions.y);

                    // Add jitter to position
                    let jitter_x: f32 = rng.random_range(-2.0..2.0);
                    let jitter_y: f32 = rng.random_range(-2.0..2.0);
                    rect.x += jitter_x;
                    rect.y += jitter_y;

                    self.cards.push(CardRect {
                        id: card.id,
                        owner_id: card.owner_id,
                        zone: card.zone.clone(),
                        tapped: card.tapped,
                        image: TextureCache::get_card_texture(&card).await,
                        rect,
                        is_hovered: false,
                        is_selected: false,
                        modifiers: card.modifiers.clone(),
                        damage_taken: card.damage_taken,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn render_grid(&mut self) {
        let grid_color = WHITE;
        let grid_thickness = 1.0;
        for cell in &self.cell_rects {
            let rect = cell.rect;
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, grid_thickness, grid_color);
            draw_text(&cell.id.to_string(), rect.x + 5.0, rect.y + 15.0, 12.0, GRAY);

            match &self.status {
                Status::SelectingZone { zones } => {
                    let intersections: Vec<&Zone> = zones
                        .iter()
                        .filter(|z| match z {
                            Zone::Intersection(locations) => locations.contains(&cell.id),
                            _ => false,
                        })
                        .collect();
                    let can_pick_intersection = !intersections.is_empty();
                    if can_pick_intersection {
                        // TODO:
                    }

                    let can_pick_zone = zones.iter().find(|i| i == &&Zone::Realm(cell.id)).is_some();
                    if can_pick_zone {
                        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 5.0, GREEN);
                    }
                }
                Status::SelectingCard { preview: true, .. } | Status::SelectingAction { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }

            // TODO: revisit this
            // if self.card_selection_overlay.is_some() {
            //     continue;
            // }

            // Draw a UI button at the top right corner as a placeholder for an icon
            let button_size = 18.0;
            let button_x = rect.x + rect.w - button_size - 4.0;
            let button_y = rect.y + 4.0;
            let button_pos = Vec2::new(button_x, button_y);
            let button_dim = Vec2::new(button_size, button_size);
            let button = ui::widgets::Button::new("+")
                .position(button_pos)
                .size(button_dim)
                .ui(&mut ui::root_ui());

            // TODO: and this
            // if button {
            //     set_clicks_enabled(false);
            //     let renderables = self
            //         .cards
            //         .iter()
            //         .filter(|c| c.zone == Zone::Realm(cell.id))
            //         .collect::<Vec<&RenderableCard>>();
            //     let prompt = format!("Viewing cards on location {}", cell.id);
            //     self.card_selection_overlay = Some(
            //         SelectionOverlay::new(
            //             self.client.clone(),
            //             &self.game_id,
            //             &self.player_id,
            //             renderables,
            //             &prompt,
            //             SelectionOverlayBehaviour::Preview,
            //         )
            //         .await,
            //     );
            // }
        }

        for intersection in &self.intersection_rects {
            match &self.status {
                Status::SelectingZone { zones } => {
                    let rect = intersection.rect;
                    let can_pick_zone = zones
                        .iter()
                        .find(|z| match z {
                            Zone::Intersection(locations) => locations == &intersection.locations,
                            _ => false,
                        })
                        .is_some();
                    if can_pick_zone {
                        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 5.0, GREEN);
                    }
                }
                Status::SelectingCard { preview: true, .. } | Status::SelectingAction { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }
    }

    fn render_background(&self) {
        draw_rectangle(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            Color::new(0.08, 0.12, 0.18, 1.0),
        );
    }
}

#[async_trait::async_trait]
impl Component for RealmComponent {
    async fn update(&mut self, cards: &[RenderableCard], status: Status) -> anyhow::Result<()> {
        self.status = status;
        self.compute_rects(cards).await
    }

    async fn render(&mut self) {
        self.render_background();
        self.render_grid().await;

        for card in &self.cards {
            if !card.zone.is_in_realm() {
                continue;
            }

            render::draw_card(card, card.owner_id == self.player_id);

            if let Status::SelectingCard {
                cards, preview: false, ..
            } = &self.status
            {
                if !clicks_enabled() {
                    return;
                }

                if !cards.contains(&card.id) {
                    draw_rectangle_ex(
                        card.rect.x,
                        card.rect.y,
                        card.rect.w * CARD_IN_PLAY_SCALE,
                        card.rect.h * CARD_IN_PLAY_SCALE,
                        DrawRectangleParams {
                            color: Color::new(100.0, 100.0, 100.0, 0.6),
                            rotation: card.rotation(),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }
}
