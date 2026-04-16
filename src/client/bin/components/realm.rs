use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{
    Color32, Context, CursorIcon, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2, epaint::Shape,
    pos2, vec2,
};
use rand::SeedableRng;
use sorcerers::{
    card::{CardData, CardType, Zone},
    game::PlayerId,
    networking::{self, message::ClientMessage},
};

static OCCUPIED_ZONE_BACKGROUND_COLOR: Color32 =
    Color32::from_rgba_unmultiplied_const(20, 31, 46, 255);

fn cell_rect(realm_rect: &Rect, id: u8, mirror: bool) -> Rect {
    let idx = id - 1;
    let mut col = idx % 5;
    let mut row = 3 - (idx / 5);
    if mirror {
        col = 4 - col;
    }
    if mirror {
        row = 3 - row;
    }
    let cell_width = realm_rect.width() / 5.0;
    let cell_height = realm_rect.height() / 4.0;
    Rect::from_min_size(
        pos2(
            realm_rect.min.x + col as f32 * cell_width,
            realm_rect.min.y + row as f32 * cell_height,
        ),
        vec2(cell_width, cell_height),
    )
}

fn intersection_rect(realm_rect: &Rect, locations: &[u8], mirror: bool) -> Option<Rect> {
    let base_rect = cell_rect(realm_rect, 1, mirror);
    let width = spell_dimensions(&base_rect).x;
    let height = spell_dimensions(&base_rect).y;
    let cell_width = realm_rect.width() / 5.0;
    let start_rect = if mirror {
        cell_rect(realm_rect, locations[locations.len() - 1], mirror)
    } else {
        cell_rect(realm_rect, locations[0], mirror)
    };
    Some(Rect::from_min_size(
        pos2(
            start_rect.min.x + cell_width - width / 2.0,
            start_rect.min.y - height / 2.0,
        ),
        vec2(width, height),
    ))
}

fn card_width(cell_rect: &Rect) -> f32 {
    cell_rect.width() / 3.5
}

fn card_height(cell_rect: &Rect) -> f32 {
    card_width(cell_rect) / CARD_ASPECT_RATIO
}

fn spell_dimensions(cell_rect: &Rect) -> Vec2 {
    vec2(card_width(cell_rect), card_height(cell_rect))
}

pub fn site_dimensions(cell_rect: &Rect) -> Vec2 {
    vec2(card_height(cell_rect), card_width(cell_rect))
}

fn card_rotation(card: &CardData) -> f32 {
    if card.tapped {
        std::f32::consts::FRAC_PI_2
    } else {
        0.0
    }
}

const CARD_FLIGHT_DURATION: f64 = 0.28;

#[derive(Clone)]
struct CardFlight {
    card_id: uuid::Uuid,
    card: CardData,
    from: Rect,
    to: Rect,
    from_rotation: f32,
    to_rotation: f32,
    started_at: f64,
    image: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for CardFlight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardFlight")
            .field("card_id", &self.card_id)
            .field("card", &self.card)
            .field("from", &self.from)
            .field("to", &self.to)
            .field("from_rotation", &self.from_rotation)
            .field("to_rotation", &self.to_rotation)
            .field("started_at", &self.started_at)
            .field("image", &self.image.as_ref().map(|_| "<TextureHandle>"))
            .finish()
    }
}

#[derive(Debug)]
pub struct RealmComponent {
    game_id: uuid::Uuid,
    player_id: PlayerId,
    cell_rects: Vec<CellRect>,
    intersection_rects: Vec<IntersectionRect>,
    card_rects: Vec<CardRect>,
    mirrored: bool,
    client: networking::client::Client,
    visible: bool,
    rect: Rect,
    last_mouse_pos: Pos2,
    card_flights: Vec<CardFlight>,
}

impl RealmComponent {
    pub fn new(
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        mirrored: bool,
        client: networking::client::Client,
        rect: Rect,
    ) -> Self {
        let cell_rects: Vec<CellRect> = (0..20)
            .map(|i| {
                let r = cell_rect(&rect, i + 1, mirrored);
                CellRect {
                    id: i as u8 + 1,
                    rect: r,
                }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    intersection_rect(&rect, &locs, mirrored).map(|r| IntersectionRect {
                        locations: locs,
                        rect: r,
                    })
                }
                _ => None,
            })
            .collect();

        Self {
            player_id: player_id.clone(),
            game_id: game_id.clone(),
            card_rects: Vec::new(),
            cell_rects,
            intersection_rects,
            mirrored,
            client,
            visible: true,
            rect,
            last_mouse_pos: pos2(0.0, 0.0),
            card_flights: Vec::new(),
        }
    }

    fn start_card_flight(
        &mut self,
        card: CardData,
        image: Option<egui::TextureHandle>,
        from_rect: Rect,
        to_rect: Rect,
        from_rotation: f32,
        to_rotation: f32,
        ctx: &Context,
    ) {
        if self
            .card_flights
            .iter()
            .any(|flight| flight.card_id == card.id)
        {
            return;
        }

        self.card_flights.push(CardFlight {
            card_id: card.id,
            card,
            from: from_rect,
            to: to_rect,
            from_rotation,
            to_rotation,
            started_at: ctx.input(|i| i.time),
            image,
        });
    }

    fn compute_rects(&mut self, cards: &[CardData], ctx: &Context) -> anyhow::Result<()> {
        use rand::Rng;

        let mut new_cards = Vec::new();
        let mut pending_flights: Vec<(
            CardData,
            Option<egui::TextureHandle>,
            Rect,
            Rect,
            f32,
            f32,
        )> = Vec::new();
        let mut rng = rand::rngs::SmallRng::seed_from_u64(1);
        for card in cards {
            let existing = self.card_rects.iter().find(|c| c.card.id == card.id);
            if let Some(existing) = existing {
                if card.zone == existing.card.zone && card.power == existing.card.power {
                    let mut new_card = existing.clone();
                    if existing.card.tapped != card.tapped && card.zone.is_in_play() {
                        pending_flights.push((
                            card.clone(),
                            new_card.image.clone(),
                            existing.rect,
                            existing.rect,
                            card_rotation(&existing.card),
                            card_rotation(card),
                        ));
                    }

                    new_card.card.tapped = card.tapped;
                    new_card.card.controller_id = card.controller_id;
                    new_card.card.power = card.power;
                    new_card.card.abilities = card.abilities.clone();
                    new_card.card.damage_taken = card.damage_taken;
                    // Update texture if not loaded yet
                    if new_card.image.is_none() {
                        new_card.image = TextureCache::get_card_texture_blocking(card, ctx);
                    }
                    new_cards.push(new_card);
                    continue;
                }
            }

            match &card.zone {
                Zone::Realm(square) => {
                    if let Some(cell) = self.cell_rects.iter().find(|c| &c.id == square) {
                        let existing = self.card_rects.iter().find(|c| c.card.id == card.id);
                        let rect = cell.rect;
                        let mut dimensions = spell_dimensions(&rect);
                        if card.card_type == CardType::Site {
                            dimensions = site_dimensions(&rect);
                        }
                        if card.is_token {
                            dimensions *= 0.7;
                        }

                        let mut pos_x = rect.min.x + (rect.width() - dimensions.x) / 2.0;
                        let mut pos_y = rect.min.y + (rect.height() - dimensions.y) / 2.0;
                        if card.card_type == CardType::Site {
                            pos_x = rect.min.x;
                            pos_y = rect.min.y + rect.height() - dimensions.y;
                        } else {
                            let jitter_x: f32 = rng.random_range(-12.0..12.0);
                            let jitter_y: f32 = rng.random_range(-12.0..12.0);
                            pos_x += jitter_x;
                            pos_y += jitter_y;
                        }

                        let selected = existing.map_or(false, |c| c.is_selected);
                        let image = existing
                            .and_then(|c| c.image.clone())
                            .or_else(|| TextureCache::get_card_texture_blocking(card, ctx));
                        if let Some(existing) = existing {
                            if existing.card.zone.is_in_play() {
                                pending_flights.push((
                                    card.clone(),
                                    image.clone(),
                                    existing.rect,
                                    Rect::from_min_size(pos2(pos_x, pos_y), dimensions),
                                    card_rotation(&existing.card),
                                    card_rotation(card),
                                ));
                            }
                        }
                        new_cards.push(CardRect {
                            image,
                            rect: Rect::from_min_size(pos2(pos_x, pos_y), dimensions),
                            is_selected: selected,
                            card: card.clone(),
                        });
                    }
                }
                Zone::Intersection(locs) => {
                    if let Some(intersection) = self
                        .intersection_rects
                        .iter()
                        .find(|c| &c.locations == locs)
                    {
                        let existing = self.card_rects.iter().find(|c| c.card.id == card.id);
                        let rect = intersection.rect;
                        let mut dimensions = spell_dimensions(&self.cell_rects[0].rect);
                        if card.card_type == CardType::Site {
                            dimensions = site_dimensions(&rect);
                        }

                        let jitter_x: f32 = rng.random_range(-2.0..2.0);
                        let jitter_y: f32 = rng.random_range(-2.0..2.0);
                        let card_rect = Rect::from_min_size(
                            pos2(rect.min.x + jitter_x, rect.min.y + jitter_y),
                            dimensions,
                        );

                        let selected = existing.map_or(false, |c| c.is_selected);
                        let image = existing
                            .and_then(|c| c.image.clone())
                            .or_else(|| TextureCache::get_card_texture_blocking(card, ctx));
                        if let Some(existing) = existing {
                            if existing.card.zone.is_in_play() {
                                pending_flights.push((
                                    card.clone(),
                                    image.clone(),
                                    existing.rect,
                                    card_rect,
                                    card_rotation(&existing.card),
                                    card_rotation(card),
                                ));
                            }
                        }
                        new_cards.push(CardRect {
                            image,
                            rect: card_rect,
                            is_selected: selected,
                            card: card.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        self.card_rects = new_cards;
        for (card, image, from_rect, to_rect, from_rotation, to_rotation) in pending_flights {
            self.start_card_flight(
                card,
                image,
                from_rect,
                to_rect,
                from_rotation,
                to_rotation,
                ctx,
            );
        }
        Ok(())
    }

    fn render_paths(&mut self, ui: &mut egui::Ui, data: &mut GameData, painter: &Painter) {
        let Status::SelectingPath { paths, .. } = &data.status.clone() else {
            return;
        };

        let mut path_points: Vec<Vec<Pos2>> = Vec::new();
        for path in paths {
            let mut points = Vec::new();
            for zone in path {
                if let Zone::Realm(id) = zone {
                    if let Some(cell_r) = self.cell_rects.iter().find(|c| c.id == *id) {
                        points.push(cell_r.rect.center());
                    }
                }
            }
            path_points.push(points);
        }

        // Use egui's Sense::click() to detect clicks on the realm area.
        let response = ui.interact(self.rect, ui.id().with("path_select"), Sense::click());
        let mouse = response.hover_pos().unwrap_or(self.last_mouse_pos);

        let mut closest_idx = None;
        let mut closest_dist = f32::MAX;
        for (idx, points) in path_points.iter().enumerate() {
            for pair in points.windows(2) {
                let (start, end) = (pair[0], pair[1]);
                let seg: Vec2 = end - start;
                let t = ((mouse - start).dot(seg) / seg.length_sq()).clamp(0.0, 1.0);
                let proj: Pos2 = start + seg * t;
                let dist = (mouse - proj).length();
                if dist < closest_dist && dist < 20.0 {
                    closest_dist = dist;
                    closest_idx = Some(idx);
                }
            }
        }

        let path_colors: [Color32; 10] = [
            Color32::from_rgb(255, 204, 51),
            Color32::from_rgb(51, 153, 255),
            Color32::from_rgb(153, 51, 255),
            Color32::from_rgb(255, 128, 0),
            Color32::from_rgb(230, 51, 255),
            Color32::from_rgb(0, 204, 255),
            Color32::from_rgb(255, 153, 179),
            Color32::from_rgb(153, 153, 255),
            Color32::from_rgb(204, 204, 77),
            Color32::from_rgb(179, 102, 255),
        ];

        for (idx, points) in path_points.iter().enumerate() {
            let color = path_colors[idx % path_colors.len()];
            let thickness = if Some(idx) == closest_idx { 4.0 } else { 1.0 };

            if points.len() >= 2 {
                for pair in points.windows(2) {
                    painter.line_segment([pair[0], pair[1]], Stroke::new(thickness, color));
                }
                let tip = points[points.len() - 1];
                let prev = points[points.len() - 2];
                let dir = (tip - prev).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let arrow_len = 12.0;
                let arrow_width = 6.0;
                let left = tip - dir * arrow_len + perp * arrow_width;
                let right = tip - dir * arrow_len - perp * arrow_width;
                painter.add(Shape::convex_polygon(
                    vec![tip, left, right],
                    color,
                    Stroke::NONE,
                ));
            }
        }

        if let Some(idx) = closest_idx {
            if response.clicked() {
                if let Status::SelectingPath { paths, .. } = &data.status {
                    if let Err(e) = self.client.send(ClientMessage::PickPath {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        path: paths[idx].clone(),
                    }) {
                        eprintln!("Error sending PickPath: {}", e);
                    }
                    data.status = Status::Idle;
                }
            }
        }
    }

    fn render_grid(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut GameData,
        painter: &Painter,
    ) -> anyhow::Result<()> {
        let grid_color = Color32::WHITE;
        let grid_thickness = 1.0;

        let occupied_zones: Vec<u8> = self
            .card_rects
            .iter()
            .filter(|c| c.card.card_type == CardType::Site)
            .filter_map(|c| match &c.card.zone {
                Zone::Realm(loc) => Some(*loc),
                _ => None,
            })
            .collect();

        let mut clicked_zone = None;
        for (idx, cell) in self.cell_rects.iter().enumerate() {
            let rect = cell.rect;
            let bg_color = if occupied_zones.contains(&cell.id) {
                OCCUPIED_ZONE_BACKGROUND_COLOR
            } else {
                Color32::from_rgba_unmultiplied(20, 31, 46, 102)
            };

            painter.rect_filled(rect, 0.0, bg_color);
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(grid_thickness, grid_color),
                egui::StrokeKind::Outside,
            );

            // Draw grid number
            painter.text(
                rect.min + vec2(10.0, 5.0),
                egui::Align2::CENTER_TOP,
                (idx + 1).to_string(),
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );

            match &data.status {
                Status::SelectingZone { zones, .. } => {
                    if zones.iter().any(|i| i == &Zone::Realm(cell.id)) {
                        let resp = ui.allocate_rect(rect, Sense::click());
                        if resp.clicked() {
                            clicked_zone = Some(Zone::Realm(cell.id));
                        }

                        painter.rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(2.5, Color32::GREEN),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                Status::DistributingDamage { .. }
                | Status::SelectingZoneGroup { .. }
                | Status::SelectingCard { preview: true, .. }
                | Status::GameAborted { .. }
                | Status::SelectingAction { .. }
                | Status::Waiting { .. }
                | Status::SelectingPath { .. }
                | Status::SelectingAmount { .. }
                | Status::Mulligan
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }

        for intersection in &self.intersection_rects {
            match &data.status {
                Status::SelectingZone { zones, .. } => {
                    let rect = intersection.rect;
                    let can_pick = zones.iter().any(|z| match z {
                        Zone::Intersection(locations) => locations == &intersection.locations,
                        _ => false,
                    });
                    if can_pick {
                        painter.rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(5.0, Color32::GREEN),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
                Status::SelectingCard { preview: true, .. }
                | Status::SelectingZoneGroup { .. }
                | Status::DistributingDamage { .. }
                | Status::Waiting { .. }
                | Status::SelectingAction { .. }
                | Status::SelectingAmount { .. }
                | Status::Mulligan
                | Status::GameAborted { .. }
                | Status::SelectingPath { .. }
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }

        if let Some(zone) = clicked_zone {
            self.zone_clicked(&zone, data)?;
        }

        let mut clicked_group_idx = None;
        if let Status::SelectingZoneGroup { groups, .. } = &data.status {
            let mouse = self.last_mouse_pos;
            let highlight_group_idx = self.cell_rects.iter().find_map(|cell_rect| {
                if !cell_rect.rect.contains(mouse) {
                    return None;
                }
                groups
                    .iter()
                    .position(|group| group.contains(&Zone::Realm(cell_rect.id)))
            });

            for (group_idx, group) in groups.iter().enumerate() {
                let base_alpha = if highlight_group_idx == Some(group_idx) {
                    179u8
                } else {
                    77u8
                };
                let color = Color32::from_rgba_unmultiplied(51, 153, 255, base_alpha);
                for zone in group {
                    if let Zone::Realm(cell_id) = zone {
                        if let Some(cell) = self.cell_rects.iter().find(|c| c.id == *cell_id) {
                            let resp = ui.allocate_rect(cell.rect, Sense::click());
                            painter.rect_filled(cell.rect, 0.0, color);

                            if resp.clicked() {
                                clicked_group_idx = Some(group_idx);
                            }
                        }
                    }
                }
            }
        }

        if let Some(group_idx) = clicked_group_idx {
            self.zone_group_clicked(group_idx, data)?;
        }

        Ok(())
    }

    fn render_prompt(&self, data: &GameData, painter: &Painter) -> anyhow::Result<()> {
        let prompt = match &data.status {
            Status::SelectingZone { prompt, .. } => Some(prompt.as_str()),
            Status::SelectingZoneGroup { prompt, .. } => Some(prompt.as_str()),
            Status::SelectingCard {
                prompt,
                preview: false,
                ..
            } => Some(prompt.as_str()),
            Status::SelectingPath { prompt, .. } => Some(prompt.as_str()),
            _ => None,
        };

        if let Some(prompt) = prompt {
            let text_size = 32.0;
            let rect_w = self.rect.width();
            let rect_h = 60.0;
            let title = painter.fonts_mut(|f| {
                f.layout_no_wrap(
                    prompt.to_string(),
                    egui::FontId::proportional(text_size),
                    Color32::WHITE,
                )
            });
            painter.rect_filled(
                Rect::from_min_size(pos2(self.rect.min.x, 0.0), vec2(rect_w, rect_h)),
                0.0,
                Color32::from_rgba_unmultiplied(38, 46, 56, 179),
            );
            painter.galley(
                pos2(self.rect.min.x + rect_w / 2.0 - title.size().x / 2.0, 5.0),
                title,
                Color32::WHITE,
            );
        }
        Ok(())
    }

    fn card_clicked(&mut self, card_id: &uuid::Uuid, data: &mut GameData) -> anyhow::Result<()> {
        let mut reset_status = false;
        match data.status.clone() {
            Status::Idle => {
                self.client.send(ClientMessage::ClickCard {
                    card_id: card_id.clone(),
                    player_id: self.player_id,
                    game_id: self.game_id,
                })?;
            }
            Status::SelectingCard {
                cards,
                multiple: false,
                ..
            } => {
                if cards.contains(card_id) {
                    self.client.send(ClientMessage::PickCard {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        card_id: card_id.clone(),
                    })?;

                    reset_status = true;
                }
            }
            Status::SelectingCard {
                cards,
                multiple: true,
                ..
            } => {
                if cards.contains(card_id) {
                    let card_rect = self
                        .card_rects
                        .iter_mut()
                        .find(|c| c.card.id == *card_id)
                        .unwrap();
                    card_rect.is_selected = !card_rect.is_selected;
                    reset_status = true;
                }
            }
            _ => {}
        }

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }

    fn zone_group_clicked(&mut self, group_idx: usize, data: &mut GameData) -> anyhow::Result<()> {
        let mut reset_status = false;
        match &data.status.clone() {
            Status::SelectingZoneGroup { groups, .. } => {
                if group_idx >= groups.len() {
                    return Ok(());
                }

                self.client.send(ClientMessage::PickZoneGroup {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    group_idx,
                })?;

                reset_status = true;
            }
            _ => {}
        }

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }

    fn zone_clicked(&mut self, zone: &Zone, data: &mut GameData) -> anyhow::Result<()> {
        let mut reset_status = false;
        match &data.status.clone() {
            Status::SelectingZone { zones, .. } => {
                if !zones.iter().any(|z| z == zone) {
                    return Ok(());
                }

                self.client.send(ClientMessage::PickZone {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    zone: zone.clone(),
                })?;

                reset_status = true;
            }
            _ => {}
        }

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }
}

impl Component for RealmComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        self.compute_rects(&data.cards, ctx)?;
        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(&self.rect, cell.id, self.mirrored);
        }

        // If a card was clicked within the last 3 seconds, find it and animate it flying to its new
        // position.
        if let Some(card_id) = data.last_clicked_card_id {
            let click_age = data
                .last_clicked_card_time
                .map(|click_time| ctx.input(|i| i.time) - click_time)
                .unwrap_or(0.0);

            if click_age > 3.0 {
                data.last_clicked_card_id = None;
                data.last_clicked_card_time = None;
            } else if let Some((card, rect, image)) = self
                .card_rects
                .iter()
                .find(|c| c.card.id == card_id && c.card.zone.is_in_play())
                .map(|card_rect| {
                    (
                        card_rect.card.clone(),
                        card_rect.rect,
                        card_rect.image.clone(),
                    )
                })
            {
                let source_center = data.last_clicked_card_pos.unwrap_or(rect.center());
                let to_rotation = card_rotation(&card);
                self.start_card_flight(
                    card,
                    image,
                    Rect::from_center_size(source_center, rect.size()),
                    rect,
                    0.0,
                    to_rotation,
                    ctx,
                );
                data.last_clicked_card_id = None;
                data.last_clicked_card_time = None;
            }
        }

        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let now = ui.ctx().input(|i| i.time);
        self.render_grid(ui, data, painter)?;

        let mut clicked_card = None;
        let mut move_delta = Vec2::default();
        let mut moved_card_id = None;
        for card_rect in &mut self.card_rects {
            if !card_rect.card.zone.is_in_play() {
                continue;
            }

            if self
                .card_flights
                .iter()
                .any(|flight| flight.card_id == card_rect.card.id)
            {
                continue;
            }

            let resp = ui.allocate_rect(card_rect.rect, Sense::HOVER | Sense::CLICK | Sense::DRAG);
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                true,
                painter,
            );

            if resp.clicked() {
                clicked_card = Some(card_rect.card.id);
            }

            if resp.hovered() {
                ui.set_cursor_icon(CursorIcon::Grab);
            }

            if resp.drag_started() {
                ui.set_cursor_icon(CursorIcon::Grab);
            }

            if resp.drag_stopped() {
                ui.set_cursor_icon(CursorIcon::Default);
            }

            if resp.hovered() {
                render::draw_card_preview(ui, card_rect.image.as_ref())?;
            }

            let cell = match &card_rect.card.zone {
                Zone::Realm(cell_id) => self.cell_rects.iter().find(|c| c.id == *cell_id),
                _ => None,
            };

            if resp.dragged() {
                if let Some(cell) = cell {
                    let card_id = card_rect.card.id;
                    let min_x = cell.rect.min.x;
                    let max_x = cell.rect.max.x - card_rect.rect.width();
                    let min_y = cell.rect.min.y;
                    let max_y = cell.rect.max.y - card_rect.rect.height();
                    let delta = resp.drag_delta();
                    let mut new_min = card_rect.rect.min + delta;
                    new_min.x = new_min.x.clamp(min_x, max_x);
                    new_min.y = new_min.y.clamp(min_y, max_y);
                    move_delta = new_min - card_rect.rect.min;
                    card_rect.rect = card_rect.rect.translate(move_delta);
                    moved_card_id = Some(card_id);
                }
            }

            if let Status::SelectingCard {
                cards,
                preview: false,
                ..
            } = &data.status
            {
                if !cards.contains(&card_rect.card.id) {
                    // Draw greying overlay
                    painter.rect_filled(
                        card_rect.rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(100, 100, 100, 153),
                    );
                }
            }
        }

        if let Some(card_id) = moved_card_id {
            let attached_cards: Vec<uuid::Uuid> = self
                .card_rects
                .iter()
                .filter(|c| c.card.bearer == Some(card_id))
                .map(|c| c.card.id)
                .collect();
            for attached_id in attached_cards {
                if let Some(ac) = self
                    .card_rects
                    .iter_mut()
                    .find(|c| c.card.id == attached_id)
                {
                    ac.rect = ac.rect.translate(move_delta);
                }
            }
        }

        if let Some(card_id) = clicked_card {
            self.card_clicked(&card_id, data)?;
        }

        self.card_flights.retain_mut(|flight| {
            let elapsed = now - flight.started_at;
            let progress = (elapsed / CARD_FLIGHT_DURATION).clamp(0.0, 1.0) as f32;
            let eased = progress * progress * (3.0 - 2.0 * progress);
            if flight.image.is_none() {
                flight.image = TextureCache::get_card_texture_blocking(&flight.card, ui.ctx());
            }
            let from = flight.from;
            let to = flight.to;
            let min = pos2(
                from.min.x + (to.min.x - from.min.x) * eased,
                from.min.y + (to.min.y - from.min.y) * eased,
            );
            let size = vec2(
                from.width() + (to.width() - from.width()) * eased,
                from.height() + (to.height() - from.height()) * eased,
            );
            let rotation =
                flight.from_rotation + (flight.to_rotation - flight.from_rotation) * eased;
            let rect = Rect::from_min_size(min, size);
            let card_rect = CardRect {
                rect,
                card: flight.card.clone(),
                image: flight.image.clone(),
                is_selected: false,
            };
            render::draw_card_with_rotation(
                &card_rect,
                card_rect.card.controller_id == self.player_id,
                true,
                painter,
                rotation,
            );
            if progress < 1.0 {
                ui.ctx().request_repaint();
                true
            } else {
                false
            }
        });

        self.render_paths(ui, data, painter);
        self.render_prompt(data, painter)?;

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        match command {
            ComponentCommand::DonePicking
                if matches!(data.status, Status::SelectingCard { .. }) =>
            {
                self.client.send(ClientMessage::PickCards {
                    game_id: self.game_id,
                    player_id: self.player_id,
                    card_ids: self
                        .card_rects
                        .iter()
                        .filter(|c| c.is_selected)
                        .map(|c| c.card.id)
                        .collect(),
                })?;
                data.status = Status::Idle;
                self.card_rects
                    .iter_mut()
                    .for_each(|c| c.is_selected = false);
            }
            ComponentCommand::SetRect {
                component_type: ComponentType::Realm,
                rect,
            } => {
                self.rect = *rect;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::Realm
    }
}
