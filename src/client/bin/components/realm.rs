use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    input::Mouse,
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{
    Color32, Context, Painter, Pos2, Rect, Stroke, Ui, Vec2, pos2, vec2,
    epaint::Shape,
};
use rand::SeedableRng;
use sorcerers::{
    card::{CardData, CardType, Zone},
    networking::{self, message::ClientMessage},
};

use std::sync::OnceLock;

static OCCUPIED_ZONE_BACKGROUND_COLOR: OnceLock<Color32> = OnceLock::new();

fn occupied_bg_color() -> Color32 {
    *OCCUPIED_ZONE_BACKGROUND_COLOR.get_or_init(|| Color32::from_rgba_unmultiplied(20, 31, 46, 255))
}

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
        pos2(realm_rect.min.x + col as f32 * cell_width, realm_rect.min.y + row as f32 * cell_height),
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
        pos2(start_rect.min.x + cell_width - width / 2.0, start_rect.min.y - height / 2.0),
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

#[derive(Debug)]
pub struct RealmComponent {
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
    cell_rects: Vec<CellRect>,
    intersection_rects: Vec<IntersectionRect>,
    card_rects: Vec<CardRect>,
    mirrored: bool,
    client: networking::client::Client,
    visible: bool,
    rect: Rect,
    last_mouse_pos: Pos2,
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
                let r = cell_rect(&rect, i + 1, mirrored);
                CellRect { id: i as u8 + 1, rect: r }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs) => {
                    intersection_rect(&rect, &locs, mirrored).map(|r| IntersectionRect { locations: locs, rect: r })
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
        }
    }

    fn compute_rects(&mut self, cards: &[CardData], ctx: &Context) -> anyhow::Result<()> {
        use rand::Rng;

        let mut new_cards = Vec::new();
        let mut rng = rand::rngs::SmallRng::seed_from_u64(1);
        for card in cards {
            let mut existing = self.card_rects.iter_mut().find(|c| c.card.id == card.id);
            if let Some(existing) = existing.as_mut() {
                if card.zone == existing.card.zone && card.power == existing.card.power {
                    existing.card.tapped = card.tapped;
                    existing.card.controller_id = card.controller_id;
                    existing.card.power = card.power;
                    existing.card.abilities = card.abilities.clone();
                    existing.card.damage_taken = card.damage_taken;
                    // Update texture if not loaded yet
                    if existing.image.is_none() {
                        existing.image = TextureCache::get_card_texture_blocking(card, ctx);
                    }
                    new_cards.push(existing.clone());
                    continue;
                }
            }

            match &card.zone {
                Zone::Realm(square) => {
                    if let Some(cell) = self.cell_rects.iter().find(|c| &c.id == square) {
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

                        let (hovered, selected) = self
                            .card_rects
                            .iter()
                            .find(|c| c.card.id == card.id)
                            .map_or((false, false), |c| (c.is_hovered, c.is_selected));
                        new_cards.push(CardRect {
                            image: TextureCache::get_card_texture_blocking(card, ctx),
                            rect: Rect::from_min_size(pos2(pos_x, pos_y), dimensions),
                            is_hovered: hovered,
                            is_selected: selected,
                            card: card.clone(),
                        });
                    }
                }
                Zone::Intersection(locs) => {
                    if let Some(intersection) = self.intersection_rects.iter().find(|c| &c.locations == locs) {
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

                        let (hovered, selected) = self
                            .card_rects
                            .iter()
                            .find(|c| c.card.id == card.id)
                            .map_or((false, false), |c| (c.is_hovered, c.is_selected));
                        new_cards.push(CardRect {
                            image: TextureCache::get_card_texture_blocking(card, ctx),
                            rect: card_rect,
                            is_hovered: hovered,
                            is_selected: selected,
                            card: card.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        self.card_rects = new_cards;
        Ok(())
    }

    fn render_paths(&self, data: &GameData, painter: &Painter) {
        match &data.status {
            Status::SelectingPath { paths, .. } => {
                let mouse = self.last_mouse_pos;

                let mut closest_idx = None;
                let mut closest_dist = f32::MAX;

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
                        painter.add(Shape::convex_polygon(vec![tip, left, right], color, Stroke::NONE));
                    }
                }
            }
            _ => {}
        }
    }

    fn render_grid(&self, data: &GameData, painter: &Painter) {
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

        for cell in &self.cell_rects {
            let rect = cell.rect;
            let bg_color = if occupied_zones.contains(&cell.id) {
                occupied_bg_color()
            } else {
                Color32::from_rgba_unmultiplied(20, 31, 46, 102)
            };

            painter.rect_filled(rect, 0.0, bg_color);
            painter.rect_stroke(rect, 0.0, Stroke::new(grid_thickness, grid_color), egui::StrokeKind::Outside);

            match &data.status {
                Status::SelectingZone { zones, .. } => {
                    if zones.iter().any(|i| i == &Zone::Realm(cell.id)) {
                        painter.rect_stroke(rect, 0.0, Stroke::new(5.0, Color32::GREEN), egui::StrokeKind::Outside);
                    }
                }
                Status::DistributingDamage { .. }
                | Status::SelectingZoneGroup { .. }
                | Status::SelectingCard { preview: true, .. }
                | Status::GameAborted { .. }
                | Status::SelectingAction { .. }
                | Status::Waiting { .. }
                | Status::SelectingPath { .. }
                | Status::Mulligan
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }

        if let Status::SelectingZoneGroup { groups, .. } = &data.status {
            let mouse = self.last_mouse_pos;
            let highlight_group_idx = self.cell_rects.iter().find_map(|cell_rect| {
                if !cell_rect.rect.contains(mouse) {
                    return None;
                }
                groups.iter().position(|group| group.contains(&Zone::Realm(cell_rect.id)))
            });

            for (group_idx, group) in groups.iter().enumerate() {
                let base_alpha = if highlight_group_idx == Some(group_idx) { 179u8 } else { 77u8 };
                let color = Color32::from_rgba_unmultiplied(51, 153, 255, base_alpha);
                for zone in group {
                    if let Zone::Realm(cell_id) = zone {
                        if let Some(cell) = self.cell_rects.iter().find(|c| c.id == *cell_id) {
                            painter.rect_filled(cell.rect, 0.0, color);
                        }
                    }
                }
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
                        painter.rect_stroke(rect, 0.0, Stroke::new(5.0, Color32::GREEN), egui::StrokeKind::Outside);
                    }
                }
                Status::SelectingCard { preview: true, .. }
                | Status::SelectingZoneGroup { .. }
                | Status::DistributingDamage { .. }
                | Status::Waiting { .. }
                | Status::SelectingAction { .. }
                | Status::Mulligan
                | Status::GameAborted { .. }
                | Status::SelectingPath { .. }
                | Status::ViewingCards { .. } => {
                    continue;
                }
                Status::SelectingCard { preview: false, .. } | Status::Idle => {}
            }
        }
    }

    fn handle_square_click(
        &mut self,
        mouse_position: Pos2,
        in_turn: bool,
        status: &mut Status,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        if !in_turn && !matches!(status, Status::SelectingZone { .. } | Status::SelectingZoneGroup { .. }) {
            return Ok(());
        }

        if let Status::SelectingAction { .. } = &status {
            return Ok(());
        }

        let clicked = Mouse::clicked(ctx);

        match &status.clone() {
            Status::SelectingZoneGroup { groups, .. } => {
                for (group_idx, group) in groups.iter().enumerate() {
                    for zone in group {
                        if let Zone::Realm(cell_id) = zone {
                            if let Some(cell) = self.cell_rects.iter().find(|c| c.id == *cell_id) {
                                if cell.rect.contains(mouse_position) && clicked {
                                    self.client.send(ClientMessage::PickZoneGroup {
                                        player_id: self.player_id,
                                        game_id: self.game_id,
                                        group_idx,
                                    })?;
                                    *status = Status::Idle;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            Status::SelectingZone { zones, .. } => {
                let zones = zones.clone();
                for (idx, cell) in self.cell_rects.iter().enumerate() {
                    let can_pick_zone = zones.iter().any(|i| i == &Zone::Realm(cell.id));
                    if !can_pick_zone {
                        continue;
                    }
                    if cell.rect.contains(mouse_position) && clicked {
                        let square = self.cell_rects[idx].id;
                        self.client.send(ClientMessage::PickZone {
                            player_id: self.player_id,
                            game_id: self.game_id,
                            zone: Zone::Realm(square),
                        })?;
                        *status = Status::Idle;
                    }
                }
                for (idx, cell) in self.intersection_rects.iter().enumerate() {
                    let can_pick = zones.iter().any(|z| match z {
                        Zone::Intersection(locations) => locations == &cell.locations,
                        _ => false,
                    });
                    if !can_pick {
                        continue;
                    }
                    if cell.rect.contains(mouse_position) && clicked {
                        let locs = self.intersection_rects[idx].locations.clone();
                        self.client.send(ClientMessage::PickZone {
                            player_id: self.player_id,
                            game_id: self.game_id,
                            zone: Zone::Intersection(locs),
                        })?;
                        *status = Status::Idle;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_card_click(
        &mut self,
        mouse_position: Pos2,
        in_turn: bool,
        data: &mut GameData,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        if !in_turn && !matches!(data.status, Status::SelectingCard { .. }) {
            return Ok(());
        }

        match &data.status {
            Status::SelectingAction { .. } | Status::SelectingZoneGroup { .. } => {
                return Ok(());
            }
            _ => {}
        }

        let mut hovered_card_index = None;
        for (idx, card) in self.card_rects.iter().enumerate() {
            if card.rect.contains(mouse_position) {
                hovered_card_index = Some(idx);
            }
        }
        for card in &mut self.card_rects {
            card.is_hovered = false;
        }
        if let Some(idx) = hovered_card_index {
            self.card_rects
                .get_mut(idx)
                .ok_or(anyhow::anyhow!("failed to get card rect"))?
                .is_hovered = true;
        }

        let clicked = Mouse::clicked(ctx);

        match data.status.clone() {
            Status::Idle => {
                for rect in self
                    .card_rects
                    .iter()
                    .filter(|c| c.card.zone.is_in_play() || c.card.zone == Zone::Hand)
                {
                    if rect.is_hovered && clicked {
                        data.last_clicked_card_pos = Some(rect.rect.center());
                        self.client.send(ClientMessage::ClickCard {
                            card_id: rect.card.id,
                            player_id: self.player_id,
                            game_id: self.game_id,
                        })?;
                    }
                }
            }
            Status::SelectingCard { cards, preview: true, .. } => {
                let mut selected_id = None;
                for card in self.card_rects.iter().filter(|c| cards.contains(&c.card.id)) {
                    if card.rect.contains(mouse_position) && clicked {
                        selected_id = Some(card.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    self.client.send(ClientMessage::PickCard {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        card_id: id,
                    })?;
                    data.status = Status::Idle;
                }
            }
            Status::SelectingCard { cards, multiple: true, preview: false, .. } => {
                let mut selected_id = None;
                for card in self.card_rects.iter().filter(|c| cards.contains(&c.card.id)) {
                    if card.rect.contains(mouse_position) && clicked {
                        selected_id = Some(card.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    let rect = self
                        .card_rects
                        .iter_mut()
                        .find(|c| c.card.id == id)
                        .ok_or(anyhow::anyhow!("failed to find card"))?;
                    rect.is_selected = !rect.is_selected;
                }
            }
            Status::SelectingCard { cards, multiple: false, preview: false, .. } => {
                let mut selected_id = None;
                for card in self.card_rects.iter().filter(|c| cards.contains(&c.card.id)) {
                    if card.rect.contains(mouse_position) && clicked {
                        selected_id = Some(card.card.id);
                    }
                }
                if let Some(id) = selected_id {
                    self.client.send(ClientMessage::PickCard {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        card_id: id,
                    })?;
                    data.status = Status::Idle;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_path_click(
        &mut self,
        mouse_position: Pos2,
        in_turn: bool,
        status: &mut Status,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        if !in_turn {
            return Ok(());
        }

        if let Status::SelectingPath { paths, .. } = &status.clone() {
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

            let mut closest_idx = None;
            let mut closest_dist = f32::MAX;
            for (idx, points) in path_points.iter().enumerate() {
                for pair in points.windows(2) {
                    let (start, end) = (pair[0], pair[1]);
                    let seg: Vec2 = end - start;
                    let t = ((mouse_position - start).dot(seg) / seg.length_sq()).clamp(0.0, 1.0);
                    let proj: Pos2 = start + seg * t;
                    let dist = (mouse_position - proj).length();
                    if dist < closest_dist && dist < 20.0 {
                        closest_dist = dist;
                        closest_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = closest_idx {
                if Mouse::clicked(ctx) {
                    self.client.send(ClientMessage::PickPath {
                        player_id: self.player_id,
                        game_id: self.game_id,
                        path: paths[idx].clone(),
                    })?;
                    *status = Status::Idle;
                }
            }
        }
        Ok(())
    }

    fn render_card_preview(&self, data: &mut GameData, painter: &Painter) -> anyhow::Result<()> {
        if let Some(card) = self.card_rects.iter().find(|card| card.is_hovered) {
            render::render_card_preview(card, data, painter)?;
        }
        Ok(())
    }

    fn render_prompt(&self, data: &GameData, painter: &Painter) -> anyhow::Result<()> {
        let prompt = match &data.status {
            Status::SelectingZone { prompt, .. } => Some(prompt.as_str()),
            Status::SelectingZoneGroup { prompt, .. } => Some(prompt.as_str()),
            Status::SelectingCard { prompt, .. } => Some(prompt.as_str()),
            Status::SelectingPath { prompt, .. } => Some(prompt.as_str()),
            _ => None,
        };

        if let Some(prompt) = prompt {
            let text_size = 32.0;
            let rect_w = self.rect.width();
            let rect_h = 60.0;
            painter.rect_filled(
                Rect::from_min_size(pos2(self.rect.min.x, 0.0), vec2(rect_w, rect_h)),
                0.0,
                Color32::from_rgba_unmultiplied(38, 46, 56, 179),
            );
            painter.text(
                pos2(self.rect.min.x + rect_w / 2.0, text_size + 5.0),
                egui::Align2::CENTER_TOP,
                prompt,
                egui::FontId::proportional(text_size),
                Color32::WHITE,
            );
        }
        Ok(())
    }
}

impl Component for RealmComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        let new_mouse_pos = Mouse::position(ctx).unwrap_or(pos2(0.0, 0.0));
        let mouse_delta: Vec2 = new_mouse_pos - self.last_mouse_pos;

        self.compute_rects(&data.cards, ctx)?;

        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(&self.rect, cell.id, self.mirrored);
        }

        let mut dragging_card: Option<uuid::Uuid> = None;
        for card in &self.card_rects {
            if card.is_hovered && Mouse::dragging(ctx) {
                dragging_card = Some(card.card.id);
            }
        }

        if let Some(card_id) = dragging_card {
            if let Some(card_rect) = self.card_rects.iter_mut().find(|c| c.card.id == card_id) {
                if let Zone::Realm(cell_id) = &card_rect.card.zone {
                    if let Some(cell) = self.cell_rects.iter().find(|c| &c.id == cell_id) {
                        let min_x = cell.rect.min.x;
                        let max_x = cell.rect.max.x - card_rect.rect.width();
                        let min_y = cell.rect.min.y;
                        let max_y = cell.rect.max.y - card_rect.rect.height();
                        let new_x = (card_rect.rect.min.x + mouse_delta.x).clamp(min_x, max_x);
                        let new_y = (card_rect.rect.min.y + mouse_delta.y).clamp(min_y, max_y);
                        let delta_x = new_x - card_rect.rect.min.x;
                        let delta_y = new_y - card_rect.rect.min.y;
                        card_rect.rect = card_rect.rect.translate(vec2(delta_x, delta_y));

                        let attached_cards: Vec<uuid::Uuid> = self
                            .card_rects
                            .iter()
                            .filter(|c| c.card.bearer == Some(card_id))
                            .map(|c| c.card.id)
                            .collect();
                        for attached_id in attached_cards {
                            if let Some(ac) = self.card_rects.iter_mut().find(|c| c.card.id == attached_id) {
                                ac.rect = ac.rect.translate(vec2(delta_x, delta_y));
                            }
                        }
                    }
                }
            }
        }

        self.last_mouse_pos = new_mouse_pos;
        Ok(())
    }

    fn render(&mut self, data: &mut GameData, _ui: &mut Ui, painter: &Painter) -> anyhow::Result<()> {
        self.render_grid(data, painter);

        for card_rect in &self.card_rects {
            if !card_rect.card.zone.is_in_play() {
                continue;
            }

            render::draw_card(card_rect, card_rect.card.controller_id == self.player_id, true, painter);

            if let Status::SelectingCard { cards, preview: false, .. } = &data.status {
                if !Mouse::enabled() {
                    continue;
                }
                if !cards.contains(&card_rect.card.id) {
                    // Draw greying overlay
                    painter.rect_filled(card_rect.rect, 0.0, Color32::from_rgba_unmultiplied(100, 100, 100, 153));
                }
            }
        }

        self.render_card_preview(data, painter)?;
        self.render_paths(data, painter);
        self.render_prompt(data, painter)?;

        Ok(())
    }

    fn process_input(&mut self, in_turn: bool, data: &mut GameData, ctx: &Context) -> anyhow::Result<Option<ComponentCommand>> {
        if !Mouse::enabled() {
            return Ok(None);
        }

        let mouse_position = Mouse::position(ctx).unwrap_or(pos2(0.0, 0.0));
        self.handle_square_click(mouse_position, in_turn, &mut data.status, ctx)?;
        self.handle_card_click(mouse_position, in_turn, data, ctx)?;
        self.handle_path_click(mouse_position, in_turn, &mut data.status, ctx)?;

        Ok(None)
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn process_command(&mut self, command: &ComponentCommand, data: &mut GameData) -> anyhow::Result<()> {
        match command {
            ComponentCommand::DonePicking if matches!(data.status, Status::SelectingCard { .. }) => {
                self.client.send(ClientMessage::PickCards {
                    game_id: self.game_id,
                    player_id: self.player_id,
                    card_ids: self.card_rects.iter().filter(|c| c.is_selected).map(|c| c.card.id).collect(),
                })?;
                data.status = Status::Idle;
                self.card_rects.iter_mut().for_each(|c| c.is_selected = false);
            }
            ComponentCommand::SetRect { component_type: ComponentType::Realm, rect } => {
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
