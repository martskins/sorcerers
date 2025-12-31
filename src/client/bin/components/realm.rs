use crate::{
    clicks_enabled,
    components::Component,
    config::{CARD_IN_PLAY_SCALE, cell_rect, intersection_rect, realm_rect, site_dimensions, spell_dimensions},
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::{game::Status, selection_overlay::SelectionOverlayBehaviour},
    set_clicks_enabled,
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, GRAY, GREEN, WHITE},
    input::{MouseButton, is_mouse_button_released},
    math::{Rect, Vec2},
    shapes::{DrawRectangleParams, draw_rectangle, draw_rectangle_ex, draw_rectangle_lines},
    text::draw_text,
    ui,
};
use rand::SeedableRng;
use sorcerers::{
    card::{CardType, RenderableCard, Zone},
    networking::{self, message::ClientMessage},
};

#[derive(Debug)]
pub struct RealmComponent {
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
    cell_rects: Vec<CellRect>,
    intersection_rects: Vec<IntersectionRect>,
    cards: Vec<CardRect>,
    mirrored: bool,
    client: networking::client::Client,
    visible: bool,
}

impl RealmComponent {
    pub fn new(
        game_id: &uuid::Uuid,
        player_id: &uuid::Uuid,
        mirrored: bool,
        client: networking::client::Client,
    ) -> Self {
        let cell_rects: Vec<CellRect> = (0..20)
            .map(|i| {
                let rect = cell_rect(i + 1, mirrored);
                CellRect { id: i as u8 + 1, rect }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    let rect = intersection_rect(&locs, mirrored).unwrap();
                    Some(IntersectionRect { locations: locs, rect })
                }
                _ => None,
            })
            .collect();

        Self {
            player_id: player_id.clone(),
            game_id: game_id.clone(),
            cards: Vec::new(),
            cell_rects,
            intersection_rects,
            mirrored,
            client,
            visible: true,
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

    async fn render_grid(&mut self, status: &mut Status) {
        let grid_color = WHITE;
        let grid_thickness = 1.0;
        for cell in &self.cell_rects {
            let rect = cell.rect;
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, grid_thickness, grid_color);
            draw_text(&cell.id.to_string(), rect.x + 5.0, rect.y + 15.0, 12.0, GRAY);

            match &status {
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
                Status::SelectingCard { preview: true, .. }
                | Status::SelectingAction { .. }
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }

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

            let to_preview = self
                .cards
                .iter()
                .filter(|c| match &c.zone {
                    Zone::Realm(loc) => loc == &cell.id,
                    _ => false,
                })
                .map(|c| c.id.clone())
                .collect::<Vec<uuid::Uuid>>();
            if button {
                set_clicks_enabled(false);
                let prompt = format!("Viewing cards on location {}", cell.id);
                let new_status = Status::ViewingCards {
                    cards: to_preview,
                    prev_status: Box::new(status.clone()),
                    prompt: prompt.clone(),
                    behaviour: SelectionOverlayBehaviour::Preview,
                };
                *status = new_status;
            }
        }

        for intersection in &self.intersection_rects {
            match &status {
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
                Status::SelectingCard { preview: true, .. }
                | Status::SelectingAction { .. }
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }
    }

    fn render_background(&self) {
        let rect = realm_rect();
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.08, 0.12, 0.18, 1.0));
    }

    fn handle_square_click(&mut self, mouse_position: Vec2, in_turn: bool, status: &mut Status) {
        if !in_turn {
            return;
        }

        if let Status::SelectingAction { .. } = &status {
            return;
        }

        match &status {
            Status::SelectingZone { zones } => {
                if !clicks_enabled() {
                    return;
                }

                let zones = zones.clone();
                for (idx, cell) in self.cell_rects.iter().enumerate() {
                    let can_pick_zone = zones.iter().find(|i| i == &&Zone::Realm(cell.id)).is_some();
                    if !can_pick_zone {
                        continue;
                    }

                    if cell.rect.contains(mouse_position.into()) {
                        let square = self.cell_rects[idx].id;
                        if is_mouse_button_released(MouseButton::Left) {
                            self.client
                                .send(ClientMessage::PickSquare {
                                    player_id: self.player_id.clone(),
                                    game_id: self.game_id.clone(),
                                    zone: Zone::Realm(square),
                                })
                                .unwrap();

                            *status = Status::Idle;
                        }
                    }
                }

                for (idx, cell) in self.intersection_rects.iter().enumerate() {
                    let can_pick_intersection = zones
                        .iter()
                        .find(|z| match z {
                            Zone::Intersection(locations) => locations == &cell.locations,
                            _ => false,
                        })
                        .is_some();
                    if !can_pick_intersection {
                        continue;
                    }

                    if cell.rect.contains(mouse_position.into()) {
                        let locs = self.intersection_rects[idx].locations.clone();
                        if is_mouse_button_released(MouseButton::Left) {
                            println!("Picking intersection at locations {:?}", cell.locations);
                            self.client
                                .send(ClientMessage::PickSquare {
                                    player_id: self.player_id.clone(),
                                    game_id: self.game_id.clone(),
                                    zone: Zone::Intersection(locs),
                                })
                                .unwrap();

                            *status = Status::Idle;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_card_click(&mut self, mouse_position: Vec2, in_turn: bool, status: &mut Status) {
        if !in_turn {
            return;
        }

        if !clicks_enabled() {
            return;
        }

        if let Status::SelectingAction { .. } = &status {
            return;
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

        match &status {
            Status::Idle => {
                for rect in &mut self
                    .cards
                    .iter_mut()
                    .filter(|c| c.zone.is_in_realm() || c.zone == Zone::Hand)
                {
                    if rect.is_hovered && is_mouse_button_released(MouseButton::Left) {
                        self.client
                            .send(ClientMessage::ClickCard {
                                card_id: rect.id.clone(),
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
                    let card = self.cards.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        *status = Status::Idle;
                    }
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
                    let card = self.cards.iter_mut().find(|c| c.id == id).unwrap();
                    card.is_selected = !card.is_selected;

                    if card.is_selected {
                        self.client
                            .send(ClientMessage::PickCard {
                                player_id: self.player_id.clone(),
                                game_id: self.game_id.clone(),
                                card_id: id.clone(),
                            })
                            .unwrap();

                        *status = Status::Idle;
                    }
                }
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl Component for RealmComponent {
    async fn update(&mut self, cards: &[RenderableCard], _status: Status) -> anyhow::Result<()> {
        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(cell.id, self.mirrored);
        }

        self.compute_rects(cards).await
    }

    async fn render(&mut self, status: &mut Status) {
        self.render_background();
        self.render_grid(status).await;

        for card in &self.cards {
            if !card.zone.is_in_realm() {
                continue;
            }

            render::draw_card(card, card.owner_id == self.player_id);

            if let Status::SelectingCard {
                cards, preview: false, ..
            } = &status
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

    fn process_input(&mut self, in_turn: bool, status: &mut Status) {
        let mouse_position = macroquad::input::mouse_position().into();
        self.handle_square_click(mouse_position, in_turn, status);
        self.handle_card_click(mouse_position, in_turn, status);
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}
