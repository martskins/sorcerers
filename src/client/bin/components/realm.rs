use crate::{
    components::{Component, ComponentCommand, ComponentType},
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
    theme,
};
use egui::{
    Color32, Context, CursorIcon, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
    epaint::Shape, pos2, vec2,
};
use rand::SeedableRng;
use sorcerers::{
    card::{CardData, CardType, Region},
    game::PlayerId,
    networking::{self, message::ClientMessage},
    zone::Zone,
};

mod geometry;

use geometry::{
    RealmViewMode, board_corners, card_rotation, cell_corners, cell_inner_rect, cell_rect,
    intersection_rect, project_rect_in_cell, projected_card_dimensions, realm_view_mode,
    set_realm_view_mode, site_dimensions, spell_dimensions,
};

static OCCUPIED_ZONE_BACKGROUND_COLOR: Color32 =
    Color32::from_rgba_unmultiplied_const(255, 255, 255, 22);

const CARD_FLIGHT_DURATION: f64 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RealmCardFilter {
    All,
    Surface,
    Underground,
    Underwater,
}

impl RealmCardFilter {
    fn label(self) -> &'static str {
        match self {
            RealmCardFilter::All => "All",
            RealmCardFilter::Surface => "Surface",
            RealmCardFilter::Underground => "Underground",
            RealmCardFilter::Underwater => "Underwater",
        }
    }

    fn includes(self, card: &CardData) -> bool {
        match self {
            RealmCardFilter::All => true,
            RealmCardFilter::Surface => {
                matches!(card.zone_region(), Some(Region::Surface | Region::Void))
            }
            RealmCardFilter::Underground => card.zone_region() == Some(Region::Underground),
            RealmCardFilter::Underwater => card.zone_region() == Some(Region::Underwater),
        }
    }
}

trait RealmCardRegion {
    fn zone_region(&self) -> Option<Region>;
}

impl RealmCardRegion for CardData {
    fn zone_region(&self) -> Option<Region> {
        match &self.zone {
            Zone::Location(_, region) | Zone::Intersection(_, region) => Some(region.clone()),
            _ => None,
        }
    }
}

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
    card_filter: RealmCardFilter,
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
                CellRect { id: i + 1, rect: r }
            })
            .collect();
        let intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs, _) => {
                    intersection_rect(&rect, &locs, mirrored).map(|r| IntersectionRect {
                        locations: locs,
                        rect: r,
                    })
                }
                _ => None,
            })
            .collect();

        Self {
            player_id: *player_id,
            game_id: *game_id,
            card_rects: Vec::new(),
            cell_rects,
            intersection_rects,
            mirrored,
            client,
            visible: true,
            rect,
            last_mouse_pos: pos2(0.0, 0.0),
            card_flights: Vec::new(),
            card_filter: RealmCardFilter::All,
        }
    }

    fn refresh_geometry(&mut self) {
        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(&self.rect, cell.id, self.mirrored);
        }

        self.intersection_rects = Zone::all_intersections()
            .into_iter()
            .filter_map(|z| match z {
                Zone::Intersection(locs, _) => intersection_rect(&self.rect, &locs, self.mirrored)
                    .map(|r| IntersectionRect {
                        locations: locs,
                        rect: r,
                    }),
                _ => None,
            })
            .collect();
    }

    #[allow(clippy::too_many_arguments)]
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
            if let Some(existing) = existing
                && card.zone == existing.card.zone
                && card.power == existing.card.power
            {
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

            match &card.zone {
                Zone::Location(square, _) => {
                    if self.cell_rects.iter().any(|c| &c.id == square) {
                        let existing = self.card_rects.iter().find(|c| c.card.id == card.id);
                        let rect = cell_inner_rect(&self.rect, *square, self.mirrored, 18.0);
                        let mut dimensions = projected_card_dimensions(
                            &self.rect,
                            *square,
                            self.mirrored,
                            card.card_type == CardType::Site,
                        );
                        if card.is_token {
                            dimensions *= 0.7;
                        }

                        let mut pos_x = rect.min.x + (rect.width() - dimensions.x) / 2.0;
                        let mut pos_y = rect.min.y + (rect.height() - dimensions.y) / 2.0;
                        if card.card_type == CardType::Site {
                            pos_y = rect.min.y + rect.height() - dimensions.y;
                        } else {
                            let jitter_x: f32 =
                                rng.random_range(-(rect.width() * 0.025)..(rect.width() * 0.025));
                            let jitter_y: f32 =
                                rng.random_range(-(rect.height() * 0.025)..(rect.height() * 0.025));
                            pos_x += jitter_x;
                            pos_y += jitter_y;
                        }
                        pos_x = pos_x.clamp(rect.min.x, rect.max.x - dimensions.x);
                        pos_y = pos_y.clamp(rect.min.y, rect.max.y - dimensions.y);

                        let selected = existing.is_some_and(|c| c.is_selected);
                        let image = existing
                            .and_then(|c| c.image.clone())
                            .or_else(|| TextureCache::get_card_texture_blocking(card, ctx));
                        if let Some(existing) = existing
                            && existing.card.zone.is_in_play()
                        {
                            pending_flights.push((
                                card.clone(),
                                image.clone(),
                                existing.rect,
                                Rect::from_min_size(pos2(pos_x, pos_y), dimensions),
                                card_rotation(&existing.card),
                                card_rotation(card),
                            ));
                        }
                        new_cards.push(CardRect {
                            image,
                            rect: Rect::from_min_size(pos2(pos_x, pos_y), dimensions),
                            is_selected: selected,
                            card: card.clone(),
                        });
                    }
                }
                Zone::Intersection(locs, _) => {
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

                        let selected = existing.is_some_and(|c| c.is_selected);
                        let image = existing
                            .and_then(|c| c.image.clone())
                            .or_else(|| TextureCache::get_card_texture_blocking(card, ctx));
                        if let Some(existing) = existing
                            && existing.card.zone.is_in_play()
                        {
                            pending_flights.push((
                                card.clone(),
                                image.clone(),
                                existing.rect,
                                card_rect,
                                card_rotation(&existing.card),
                                card_rotation(card),
                            ));
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
                if let Zone::Location(id, _) = zone
                    && let Some(cell_r) = self.cell_rects.iter().find(|c| c.id == *id)
                {
                    points.push(cell_r.rect.center());
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

        if let Some(idx) = closest_idx
            && response.clicked()
            && let Status::SelectingPath { paths, .. } = &data.status
        {
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

    fn draw_playmat(&self, painter: &Painter) {
        let corners = board_corners(&self.rect).to_vec();
        painter.add(Shape::convex_polygon(
            corners.clone(),
            Color32::from_rgb(25, 27, 25),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 195, 160, 65)),
        ));
    }

    fn draw_zone_guide(&self, painter: &Painter, cell_id: u8, occupied: bool) {
        let guide = cell_corners(&self.rect, cell_id, self.mirrored, 1.5);
        if occupied {
            painter.add(Shape::convex_polygon(
                guide.to_vec(),
                OCCUPIED_ZONE_BACKGROUND_COLOR,
                Stroke::NONE,
            ));
        }

        painter.add(Shape::closed_line(
            guide.to_vec(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(235, 235, 220, 42)),
        ));

        let number_pos = guide[0] + (guide[1] - guide[0]) * 0.08 + (guide[3] - guide[0]) * 0.16;
        painter.text(
            number_pos,
            egui::Align2::LEFT_TOP,
            cell_id.to_string(),
            egui::FontId::proportional(11.0),
            Color32::from_rgba_unmultiplied(235, 235, 220, 100),
        );
    }

    fn render_grid(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut GameData,
        painter: &Painter,
    ) -> anyhow::Result<()> {
        let occupied_zones: Vec<u8> = self
            .filtered_card_rects()
            .filter(|c| c.card.card_type == CardType::Site)
            .filter_map(|c| match &c.card.zone {
                Zone::Location(loc, _) => Some(*loc),
                _ => None,
            })
            .collect();

        self.draw_playmat(painter);

        let mut clicked_zone = None;
        for cell in &self.cell_rects {
            let rect = cell.rect;

            self.draw_zone_guide(painter, cell.id, occupied_zones.contains(&cell.id));

            let playable_preview_zones = match &data.status {
                Status::SelectingZone { zones, .. }
                | Status::PreviewingPlayableZones { zones, .. } => Some(zones),
                _ => None,
            };
            if let Some(zones) = playable_preview_zones {
                if let Some(zone) = zones
                    .iter()
                    .find(|zone| matches!(zone, Zone::Location(id, _) if *id == cell.id))
                {
                    if matches!(data.status, Status::SelectingZone { .. }) {
                        let resp = ui.allocate_rect(rect, Sense::click());
                        if resp.clicked() {
                            clicked_zone = Some(zone.clone());
                        }
                    }

                    painter.add(Shape::closed_line(
                        cell_corners(&self.rect, cell.id, self.mirrored, 5.0).to_vec(),
                        Stroke::new(3.0, theme::PICKABLE),
                    ));
                }
            } else {
                match &data.status {
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
                    Status::SelectingZone { .. } | Status::PreviewingPlayableZones { .. } => {}
                }
            }
        }

        for intersection in &self.intersection_rects {
            let playable_preview_zones = match &data.status {
                Status::SelectingZone { zones, .. }
                | Status::PreviewingPlayableZones { zones, .. } => Some(zones),
                _ => None,
            };
            if let Some(zones) = playable_preview_zones {
                let rect = intersection.rect;
                let can_pick = zones.iter().any(|z| match z {
                    Zone::Intersection(locations, _) => locations == &intersection.locations,
                    _ => false,
                });
                if can_pick {
                    painter.rect_stroke(
                        rect,
                        4.0,
                        Stroke::new(3.0, theme::PICKABLE),
                        egui::StrokeKind::Outside,
                    );
                }
            } else {
                match &data.status {
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
                    Status::SelectingZone { .. } | Status::PreviewingPlayableZones { .. } => {}
                }
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
                groups.iter().position(|group| {
                    group
                        .iter()
                        .any(|zone| matches!(zone, Zone::Location(id, _) if *id == cell_rect.id))
                })
            });

            for (group_idx, group) in groups.iter().enumerate() {
                let base_alpha = if highlight_group_idx == Some(group_idx) {
                    179u8
                } else {
                    77u8
                };
                let color = Color32::from_rgba_unmultiplied(80, 200, 190, base_alpha);
                for zone in group {
                    if let Zone::Location(cell_id, _) = zone
                        && let Some(cell) = self.cell_rects.iter().find(|c| c.id == *cell_id)
                    {
                        let resp = ui.allocate_rect(cell.rect, Sense::click());
                        painter.add(Shape::convex_polygon(
                            cell_corners(&self.rect, cell.id, self.mirrored, 5.0).to_vec(),
                            color,
                            Stroke::NONE,
                        ));

                        if resp.clicked() {
                            clicked_group_idx = Some(group_idx);
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

    fn render_prompt(&self, _data: &GameData, _painter: &Painter) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_view_controls(&mut self, ui: &mut Ui) {
        let button_pos = pos2(self.rect.min.x + 18.0, self.rect.min.y + 18.0);
        egui::Area::new(egui::Id::new("realm_view_controls"))
            .fixed_pos(button_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let mode = realm_view_mode();
                    let next_mode = match mode {
                        RealmViewMode::Perspective3d => RealmViewMode::TopDown2d,
                        RealmViewMode::TopDown2d => RealmViewMode::Perspective3d,
                    };
                    let button_label = match mode {
                        RealmViewMode::Perspective3d => "3D View",
                        RealmViewMode::TopDown2d => "2D View",
                    };

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(button_label)
                                    .size(14.0)
                                    .color(theme::TEXT_BRIGHT),
                            )
                            .min_size(vec2(84.0, 30.0)),
                        )
                        .on_hover_text("Switch realm view")
                        .clicked()
                    {
                        set_realm_view_mode(next_mode);
                        self.refresh_geometry();
                        self.card_rects.clear();
                    }

                    ui.add_space(8.0);

                    for filter in [
                        RealmCardFilter::All,
                        RealmCardFilter::Surface,
                        RealmCardFilter::Underground,
                        RealmCardFilter::Underwater,
                    ] {
                        let selected = self.card_filter == filter;
                        if ui
                            .selectable_label(selected, filter.label())
                            .on_hover_text("Filter cards shown on the realm")
                            .clicked()
                        {
                            self.card_filter = filter;
                        }
                    }
                });
            });
    }

    fn card_visible_in_filter(&self, card: &CardData) -> bool {
        self.card_filter.includes(card)
    }

    fn filtered_card_rects(&self) -> impl Iterator<Item = &CardRect> {
        self.card_rects
            .iter()
            .filter(|card_rect| self.card_visible_in_filter(&card_rect.card))
    }

    fn clear_flights_hidden_by_filter(&mut self) {
        let card_filter = self.card_filter;
        self.card_flights
            .retain(|flight| card_filter.includes(&flight.card));
    }

    fn card_clicked(&mut self, card_id: &uuid::Uuid, data: &mut GameData) -> anyhow::Result<()> {
        let mut reset_status = false;
        match data.status.clone() {
            Status::Idle => {
                self.client.send(ClientMessage::ClickCard {
                    card_id: *card_id,
                    player_id: self.player_id,
                    game_id: self.game_id,
                })?;
            }
            Status::SelectingCard {
                cards,
                multiple: false,
                ..
            } if cards.contains(card_id) => {
                self.client.send(ClientMessage::PickCard {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_id: *card_id,
                })?;

                reset_status = true;
            }
            Status::SelectingCard {
                cards: _,
                multiple: false,
                ..
            } => {}
            Status::SelectingCard {
                cards,
                multiple: true,
                ..
            } if cards.contains(card_id) => {
                if let Some(card_rect) = self.card_rects.iter_mut().find(|c| c.card.id == *card_id)
                {
                    card_rect.is_selected = !card_rect.is_selected;
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
        if let Status::SelectingZoneGroup { groups, .. } = &data.status.clone() {
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

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }

    fn zone_clicked(&mut self, zone: &Zone, data: &mut GameData) -> anyhow::Result<()> {
        let mut reset_status = false;
        if let Status::SelectingZone { zones, .. } = &data.status.clone() {
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

        if reset_status {
            data.status = Status::Idle;
        }

        Ok(())
    }
}

impl Component for RealmComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        self.refresh_geometry();
        self.compute_rects(&data.cards, ctx)?;

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
        let realm_rect = self.rect;
        let mirrored = self.mirrored;
        let card_filter = self.card_filter;
        for card_rect in &mut self.card_rects {
            if !card_rect.card.zone.is_in_play() {
                continue;
            }

            if !card_filter.includes(&card_rect.card) {
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
            if let Zone::Location(cell_id, _) = card_rect.card.zone {
                let corners = project_rect_in_cell(
                    &realm_rect,
                    cell_id,
                    mirrored,
                    card_rect.rect,
                    18.0,
                    card_rotation(&card_rect.card),
                );
                render::draw_projected_card(
                    card_rect,
                    card_rect.card.controller_id == self.player_id,
                    true,
                    painter,
                    corners,
                );
            } else {
                render::draw_card(
                    card_rect,
                    card_rect.card.controller_id == self.player_id,
                    true,
                    painter,
                );
            }

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
                Zone::Location(cell_id, _) => self.cell_rects.iter().find(|c| c.id == *cell_id),
                _ => None,
            };

            if resp.dragged()
                && let Some(cell) = cell
            {
                let card_id = card_rect.card.id;
                let bounds = match card_rect.card.zone {
                    Zone::Location(cell_id, _) => {
                        cell_inner_rect(&realm_rect, cell_id, mirrored, 18.0)
                    }
                    _ => cell.rect,
                };
                let min_x = bounds.min.x;
                let max_x = bounds.max.x - card_rect.rect.width();
                let min_y = bounds.min.y;
                let max_y = bounds.max.y - card_rect.rect.height();
                let delta = resp.drag_delta();
                let mut new_min = card_rect.rect.min + delta;
                new_min.x = new_min.x.clamp(min_x, max_x);
                new_min.y = new_min.y.clamp(min_y, max_y);
                move_delta = new_min - card_rect.rect.min;
                card_rect.rect = card_rect.rect.translate(move_delta);
                moved_card_id = Some(card_id);
            }

            if let Status::SelectingCard {
                cards,
                preview: false,
                ..
            } = &data.status
                && !cards.contains(&card_rect.card.id)
            {
                let overlay_color = Color32::from_rgba_unmultiplied(100, 100, 100, 153);
                if let Zone::Location(cell_id, _) = card_rect.card.zone {
                    painter.add(Shape::convex_polygon(
                        project_rect_in_cell(
                            &realm_rect,
                            cell_id,
                            mirrored,
                            card_rect.rect,
                            18.0,
                            card_rotation(&card_rect.card),
                        )
                        .to_vec(),
                        overlay_color,
                        Stroke::NONE,
                    ));
                } else {
                    painter.rect_filled(card_rect.rect, 0.0, overlay_color);
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

        self.clear_flights_hidden_by_filter();
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
            if let Zone::Location(cell_id, _) = card_rect.card.zone {
                render::draw_projected_card(
                    &card_rect,
                    card_rect.card.controller_id == self.player_id,
                    true,
                    painter,
                    project_rect_in_cell(
                        &self.rect,
                        cell_id,
                        self.mirrored,
                        card_rect.rect,
                        18.0,
                        rotation,
                    ),
                );
            } else {
                render::draw_card_with_rotation(
                    &card_rect,
                    card_rect.card.controller_id == self.player_id,
                    true,
                    painter,
                    rotation,
                );
            }
            if progress < 1.0 {
                ui.ctx().request_repaint();
                true
            } else {
                false
            }
        });

        self.render_paths(ui, data, painter);
        self.render_prompt(data, painter)?;
        self.render_view_controls(ui);

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
                self.refresh_geometry();
            }
            ComponentCommand::DropHandCard { card_id, pos } => {
                if let Status::PreviewingPlayableZones {
                    card_id: preview_card_id,
                    zones,
                } = &data.status.clone()
                    && preview_card_id == card_id
                {
                    let dropped_zone = self
                        .cell_rects
                        .iter()
                        .find(|cell| cell.rect.contains(*pos))
                        .and_then(|cell| {
                            zones.iter().find(
                                |zone| matches!(zone, Zone::Location(id, _) if *id == cell.id),
                            )
                        })
                        .or_else(|| {
                            self.intersection_rects
                                .iter()
                                .find(|intersection| intersection.rect.contains(*pos))
                                .and_then(|intersection| {
                                    zones.iter().find(|zone| {
                                        matches!(
                                            zone,
                                            Zone::Intersection(locations, _)
                                                if locations == &intersection.locations
                                        )
                                    })
                                })
                        });

                    if let Some(zone) = dropped_zone {
                        self.client.send(ClientMessage::PlayCardAtZone {
                            player_id: self.player_id,
                            game_id: self.game_id,
                            card_id: *card_id,
                            zone: zone.clone(),
                        })?;
                    }
                    data.status = Status::Idle;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::Realm
    }
}
