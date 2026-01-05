use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    input::Mouse,
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::{
        game::{GameData, Status},
        selection_overlay::SelectionOverlayBehaviour,
    },
    texture_cache::TextureCache,
};
use macroquad::{
    color::{Color, GRAY, GREEN, WHITE},
    input::{MouseButton, is_mouse_button_released, mouse_position},
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

fn cell_rect(realm_rect: &Rect, id: u8, mirror: bool) -> Rect {
    // The grid layout looks like the following from player's 1 perspective (for player 2, the
    // board is flipped both horizontally and vertically):
    // ________________________
    // |16 | 17 | 18 | 19 | 20 |
    // |---|----|----|----|----|
    // |11 | 12 | 13 | 14 | 15 |
    // |---|----|----|----|----|
    // |6  | 7  | 8  | 9  | 10 |
    // |---|----|----|----|----|
    // |1  | 2  | 3  | 4  | 5  |
    // |-----------------------|
    let idx = id - 1;
    let mut col = idx % 5;
    let mut row = 3 - (idx / 5); // invert row for bottom-up indexing

    if mirror {
        col = 4 - col; // mirror horizontally
    }
    if mirror {
        row = 3 - row; // mirror vertically
    }

    Rect::new(
        realm_rect.x + col as f32 * (realm_rect.w / 5.0),
        realm_rect.y + row as f32 * (realm_rect.h / 4.0),
        realm_rect.w / 5.0,
        realm_rect.h / 4.0,
    )
}

fn intersection_rect(realm_rect: &Rect, locations: &[u8], mirror: bool) -> Option<Rect> {
    let base_rect = cell_rect(realm_rect, 1, mirror);
    let width = spell_dimensions(&base_rect).x;
    let height = spell_dimensions(&base_rect).y;
    let cell_width = realm_rect.w / 5.0;
    let start_rect = if mirror {
        cell_rect(realm_rect, locations[locations.len() - 1], mirror)
    } else {
        cell_rect(realm_rect, locations[0], mirror)
    };
    Some(Rect::new(
        start_rect.x + cell_width - width / 2.0,
        start_rect.y - height / 2.0,
        width,
        height,
    ))
}

fn card_width(cell_rect: &Rect) -> f32 {
    cell_rect.w / 3.5
}

fn card_height(cell_rect: &Rect) -> f32 {
    card_width(cell_rect) / CARD_ASPECT_RATIO
}

fn spell_dimensions(cell_rect: &Rect) -> Vec2 {
    Vec2::new(card_width(cell_rect), card_height(cell_rect))
}

pub fn site_dimensions(cell_rect: &Rect) -> Vec2 {
    Vec2::new(card_height(cell_rect), card_width(cell_rect))
}

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
    rect: Rect,
    last_mouse_pos: Vec2,
}

impl RealmComponent {
    pub fn new(
        game_id: &uuid::Uuid,
        player_id: &uuid::Uuid,
        mirrored: bool,
        client: networking::client::Client,
        rect: Rect,
    ) -> Self {
        let cell_rects: Vec<CellRect> = (0..20)
            .map(|i| {
                let rect = cell_rect(&rect, i + 1, mirrored);
                CellRect { id: i as u8 + 1, rect }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    let rect = intersection_rect(&rect, &locs, mirrored).unwrap();
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
            rect,
            last_mouse_pos: mouse_position().into(),
        }
    }

    async fn compute_rects(&mut self, cards: &[RenderableCard]) -> anyhow::Result<()> {
        use rand::Rng;

        let mut new_cards = Vec::new();
        let mut rng = rand::rngs::SmallRng::seed_from_u64(1);
        for card in cards {
            let mut existing_card = self.cards.iter_mut().find(|c| c.id == card.id);
            if let Some(existing_card) = existing_card.as_mut() {
                if card.zone == existing_card.zone {
                    existing_card.tapped = card.tapped;
                    existing_card.modifiers = card.modifiers.clone();
                    existing_card.damage_taken = card.damage_taken;
                    new_cards.push(existing_card.clone());
                    continue;
                }
            }

            match &card.zone {
                Zone::Realm(square) => {
                    let cell_rect = self.cell_rects.iter().find(|c| &c.id == square).unwrap().rect;
                    let mut dimensions = spell_dimensions(&cell_rect);
                    if card.card_type == CardType::Site {
                        dimensions = site_dimensions(&cell_rect);
                    }

                    let mut rect = Rect::new(
                        cell_rect.x + (cell_rect.w - dimensions.x) / 2.0,
                        cell_rect.y + (cell_rect.h - dimensions.y) / 2.0,
                        dimensions.x,
                        dimensions.y,
                    );

                    // Add jitter to position
                    let jitter_x: f32 = rng.random_range(-12.0..12.0);
                    let jitter_y: f32 = rng.random_range(-12.0..12.0);
                    rect.x += jitter_x;
                    rect.y += jitter_y;

                    new_cards.push(CardRect {
                        id: card.id,
                        owner_id: card.owner_id,
                        zone: card.zone.clone(),
                        tapped: card.tapped,
                        image: TextureCache::get_card_texture(&card).await,
                        rect,
                        is_hovered: existing_card.map_or(false, |c| c.is_hovered),
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
                    // Use a grid cell as a reference for the spell dimensions instead of the
                    // intersection rect itself, as the intersection rect is smaller and the card
                    // would be too small.
                    let mut dimensions = spell_dimensions(&self.cell_rects[0].rect);
                    if card.card_type == CardType::Site {
                        dimensions = site_dimensions(&rect);
                    }

                    let mut rect = Rect::new(rect.x, rect.y, dimensions.x, dimensions.y);

                    // Add jitter to position
                    let jitter_x: f32 = rng.random_range(-2.0..2.0);
                    let jitter_y: f32 = rng.random_range(-2.0..2.0);
                    rect.x += jitter_x;
                    rect.y += jitter_y;

                    new_cards.push(CardRect {
                        id: card.id,
                        owner_id: card.owner_id,
                        zone: card.zone.clone(),
                        tapped: card.tapped,
                        image: TextureCache::get_card_texture(&card).await,
                        rect,
                        is_hovered: self
                            .cards
                            .iter()
                            .find(|c| c.id == card.id)
                            .map_or(false, |c| c.is_hovered),
                        modifiers: card.modifiers.clone(),
                        damage_taken: card.damage_taken,
                    });
                }
                _ => {}
            }
        }

        self.cards = new_cards;

        Ok(())
    }

    fn render_paths(&mut self, data: &mut GameData) {
        use macroquad::input::mouse_position;

        match &data.status {
            Status::SelectingPath { paths } => {
                let mouse = {
                    let (mx, my) = mouse_position();
                    Vec2::new(mx, my)
                };

                // Find the path closest to the mouse (by minimum distance to any segment)
                let mut closest_idx = None;
                let mut closest_dist = f32::MAX;

                let mut path_points: Vec<Vec<Vec2>> = Vec::new();

                for path in paths {
                    let mut points = Vec::new();
                    for zone in path {
                        if let Zone::Realm(id) = zone {
                            if let Some(cell_rect) = self.cell_rects.iter().find(|c| c.id == *id) {
                                let center = Vec2::new(
                                    cell_rect.rect.x + cell_rect.rect.w / 2.0,
                                    cell_rect.rect.y + cell_rect.rect.h / 2.0,
                                );
                                points.push(center);
                            }
                        }
                    }
                    path_points.push(points);
                }

                for (idx, points) in path_points.iter().enumerate() {
                    for pair in points.windows(2) {
                        let (start, end) = (pair[0], pair[1]);
                        // Distance from mouse to segment
                        let seg = end - start;
                        let t = ((mouse - start).dot(seg) / seg.length_squared()).clamp(0.0, 1.0);
                        let proj = start + seg * t;
                        let dist = (mouse - proj).length();
                        if dist < closest_dist && dist < 20.0 {
                            closest_dist = dist;
                            closest_idx = Some(idx);
                        }
                    }
                }

                let path_colors: [Color; 10] = [
                    Color::new(1.0, 0.8, 0.2, 1.0), // yellow
                    Color::new(0.2, 0.6, 1.0, 1.0), // blue
                    Color::new(0.6, 0.2, 1.0, 1.0), // purple
                    Color::new(1.0, 0.5, 0.0, 1.0), // orange
                    Color::new(0.9, 0.2, 1.0, 1.0), // magenta
                    Color::new(0.0, 0.8, 1.0, 1.0), // cyan
                    Color::new(1.0, 0.6, 0.7, 1.0), // pink
                    Color::new(0.6, 0.6, 1.0, 1.0), // light blue
                    Color::new(0.8, 0.8, 0.3, 1.0), // light yellow
                    Color::new(0.7, 0.4, 1.0, 1.0), // violet
                ];

                for (idx, points) in path_points.iter().enumerate() {
                    let color = path_colors[idx % path_colors.len()];
                    let thickness = if Some(idx) == closest_idx { 4.0 } else { 1.0 };

                    if points.len() >= 2 {
                        // Draw lines between consecutive points
                        for pair in points.windows(2) {
                            let (start, end) = (pair[0], pair[1]);
                            macroquad::shapes::draw_line(start.x, start.y, end.x, end.y, thickness, color);
                        }
                        // Draw arrowhead at the end
                        let tip = points[points.len() - 1];
                        let prev = points[points.len() - 2];
                        let dir = (tip - prev).normalize();
                        let perp = Vec2::new(-dir.y, dir.x);
                        let arrow_len = 12.0;
                        let arrow_width = 6.0;
                        let left = tip - dir * arrow_len + perp * arrow_width;
                        let right = tip - dir * arrow_len - perp * arrow_width;
                        macroquad::shapes::draw_triangle(tip, left, right, color);
                    }
                }
            }
            _ => {}
        }
    }

    async fn render_grid(&mut self, data: &mut GameData) {
        let grid_color = WHITE;
        let grid_thickness = 1.0;
        for cell in &self.cell_rects {
            let rect = cell.rect;
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, grid_thickness, grid_color);
            draw_text(&cell.id.to_string(), rect.x + 5.0, rect.y + 15.0, 12.0, GRAY);

            match &data.status {
                Status::SelectingZone { zones } => {
                    let can_pick_zone = zones.iter().find(|i| i == &&Zone::Realm(cell.id)).is_some();
                    if can_pick_zone {
                        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 5.0, GREEN);
                    }
                }
                Status::SelectingCard { preview: true, .. }
                | Status::SelectingAction { .. }
                | Status::Waiting { .. }
                | Status::SelectingPath { .. }
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
                Mouse::set_enabled(false);
                let prompt = format!("Viewing cards on location {}", cell.id);
                let new_status = Status::ViewingCards {
                    cards: to_preview,
                    prev_status: Box::new(data.status.clone()),
                    prompt: prompt.clone(),
                    behaviour: SelectionOverlayBehaviour::Preview,
                };
                data.status = new_status;
            }
        }

        for intersection in &self.intersection_rects {
            match &data.status {
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
                | Status::Waiting { .. }
                | Status::SelectingAction { .. }
                | Status::SelectingPath { .. }
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }
    }

    fn render_background(&self) {
        let rect = self.rect;
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
                if !Mouse::is_enabled() {
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
                                .send(ClientMessage::PickZone {
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
                            self.client
                                .send(ClientMessage::PickZone {
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

        if !Mouse::is_enabled() {
            return;
        }

        if let Status::SelectingAction { .. } = &status {
            return;
        }

        let mut hovered_card_index = None;
        for (idx, card) in self.cards.iter().enumerate() {
            if card.rect.contains(mouse_position.into()) {
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
                    if rect.is_hovered && Mouse::is_clicked() {
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
                    if card.rect.contains(mouse_position.into()) && Mouse::is_clicked() {
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

                    *status = Status::Idle;
                }
            }

            Status::SelectingCard {
                cards, preview: false, ..
            } => {
                let valid_cards: Vec<&CardRect> = self.cards.iter().filter(|c| cards.contains(&c.id)).collect();
                let mut selected_id = None;
                for card in valid_cards {
                    if card.rect.contains(mouse_position.into()) && Mouse::is_clicked() {
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

                    *status = Status::Idle;
                }
            }
            _ => {}
        }
    }

    fn handle_path_click(&mut self, mouse_position: Vec2, in_turn: bool, status: &mut Status) {
        use macroquad::input::is_mouse_button_released;
        if !in_turn {
            return;
        }

        if let Status::SelectingPath { paths } = &status {
            let mut path_points: Vec<Vec<Vec2>> = Vec::new();
            for path in paths {
                let mut points = Vec::new();
                for zone in path {
                    if let Zone::Realm(id) = zone {
                        if let Some(cell_rect) = self.cell_rects.iter().find(|c| c.id == id.clone()) {
                            let center = Vec2::new(
                                cell_rect.rect.x + cell_rect.rect.w / 2.0,
                                cell_rect.rect.y + cell_rect.rect.h / 2.0,
                            );
                            points.push(center);
                        }
                    }
                }
                path_points.push(points);
            }

            // Find closest path to mouse
            let mut closest_idx = None;
            let mut closest_dist = f32::MAX;
            for (idx, points) in path_points.iter().enumerate() {
                for pair in points.windows(2) {
                    let (start, end) = (pair[0], pair[1]);
                    let seg = end - start;
                    let t = ((mouse_position - start).dot(seg) / seg.length_squared()).clamp(0.0, 1.0);
                    let proj = start + seg * t;
                    let dist = (mouse_position - proj).length();
                    if dist < closest_dist && dist < 20.0 {
                        closest_dist = dist;
                        closest_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = closest_idx {
                if is_mouse_button_released(MouseButton::Left) {
                    self.client
                        .send(ClientMessage::PickPath {
                            player_id: self.player_id.clone(),
                            game_id: self.game_id.clone(),
                            path: paths[idx].clone(),
                        })
                        .unwrap();
                    *status = Status::Idle;
                }
            }
        }
    }

    async fn render_card_preview(&self, data: &mut GameData) -> anyhow::Result<()> {
        if let Some(card) = self.cards.iter().find(|card| card.is_hovered) {
            render::render_card_preview(card, data).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Component for RealmComponent {
    async fn update(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        let new_mouse_pos: Vec2 = mouse_position().into();
        let mouse_delta = new_mouse_pos - self.last_mouse_pos;

        self.compute_rects(&data.cards).await?;

        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(&self.rect, cell.id, self.mirrored);
        }

        let mut dragging_card: Option<uuid::Uuid> = None;
        for card in &mut self.cards {
            if card.is_hovered && Mouse::dragging() {
                dragging_card = Some(card.id.clone());
            }
        }

        if let Some(card_id) = dragging_card {
            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                if let Zone::Realm(cell_id) = &card.zone {
                    if let Some(cell) = self.cell_rects.iter().find(|c| &c.id == cell_id) {
                        let min_x = cell.rect.x;
                        let max_x = cell.rect.x + cell.rect.w - card.rect.w;
                        let min_y = cell.rect.y;
                        let max_y = cell.rect.y + cell.rect.h - card.rect.h;
                        card.rect.x = (card.rect.x + mouse_delta.x).clamp(min_x, max_x);
                        card.rect.y = (card.rect.y + mouse_delta.y).clamp(min_y, max_y);
                    }
                }
            }
        }

        self.last_mouse_pos = mouse_position().into();
        Ok(())
    }

    async fn render(&mut self, data: &mut GameData) -> anyhow::Result<()> {
        self.render_background();
        self.render_grid(data).await;

        for card in &mut self.cards {
            if !card.zone.is_in_realm() {
                continue;
            }

            render::draw_card(card, card.owner_id == self.player_id);

            if let Status::SelectingCard {
                cards, preview: false, ..
            } = &data.status
            {
                if !Mouse::is_enabled() {
                    return Ok(());
                }

                if !cards.contains(&card.id) {
                    draw_rectangle_ex(
                        card.rect.x,
                        card.rect.y,
                        card.rect.w,
                        card.rect.h,
                        DrawRectangleParams {
                            color: Color::new(100.0, 100.0, 100.0, 0.6),
                            rotation: card.rotation(),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        self.render_card_preview(data).await?;
        self.render_paths(data);

        Ok(())
    }

    fn process_input(&mut self, in_turn: bool, data: &mut GameData) -> anyhow::Result<Option<ComponentCommand>> {
        let mouse_position = macroquad::input::mouse_position().into();
        self.handle_square_click(mouse_position, in_turn, &mut data.status);
        self.handle_card_click(mouse_position, in_turn, &mut data.status);
        self.handle_path_click(mouse_position, in_turn, &mut data.status);

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    async fn process_command(&mut self, command: &ComponentCommand) {
        match command {
            ComponentCommand::SetRect {
                component_type: ComponentType::Realm,
                rect,
            } => {
                self.rect = rect.clone();
            }
            _ => {}
        }
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::Realm
    }
}
