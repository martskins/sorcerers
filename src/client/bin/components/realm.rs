use crate::{
    components::{Component, ComponentCommand, ComponentType},
    render::{self, CardRect, CellRect, IntersectionRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
    theme,
};
use egui::{
    Color32, Context, CursorIcon, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2, epaint::Shape,
    pos2, vec2,
};
use rand::SeedableRng;
use sorcerers::{
    card::{CardData, CardType, Region},
    game::{CardId, Direction, PlayerId},
    networking::{
        self,
        message::{ClientMessage, OngoingEffectData},
    },
    zone::{Location, Zone},
};

mod geometry;

use geometry::{
    board_corners, card_corners, card_rotation, cell_corners, cell_inner_rect, cell_rect,
    intersection_rect, projected_card_dimensions, site_dimensions,
};

static OCCUPIED_ZONE_BACKGROUND_COLOR: Color32 =
    Color32::from_rgba_unmultiplied_const(255, 255, 255, 22);

const CARD_FLIGHT_DURATION: f64 = 0.28;
const PROJECTILE_FLIGHT_DURATION: f64 = 0.42;

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
            Zone::Location(Location::Square(_, region))
            | Zone::Location(Location::Intersection(_, region)) => Some(region.clone()),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct CardFlight {
    card_id: CardId,
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

#[derive(Debug, Clone)]
struct ProjectileFlight {
    id: uuid::Uuid,
    points: Vec<Pos2>,
    direction: Direction,
    started_at: f64,
    ranged_strike: bool,
}

#[derive(Debug, Clone)]
enum PendingLocationChoiceAction {
    PickLocation,
    PlayHandCard { card_id: CardId },
}

#[derive(Debug, Clone)]
struct PendingLocationChoice {
    pos: Pos2,
    locations: Vec<Location>,
    action: PendingLocationChoiceAction,
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
    projectile_flights: Vec<ProjectileFlight>,
    card_filter: RealmCardFilter,
    pending_zone_choice: Option<PendingLocationChoice>,
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
        let intersection_rects = Location::all_intersections()
            .into_iter()
            .filter_map(|location| match location {
                Location::Intersection(locs, _) => {
                    intersection_rect(&rect, &locs, mirrored).map(|r| IntersectionRect {
                        locations: locs,
                        rect: r,
                    })
                }
                Location::Square(_, _) => None,
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
            projectile_flights: Vec::new(),
            card_filter: RealmCardFilter::All,
            pending_zone_choice: None,
        }
    }

    fn refresh_geometry(&mut self) {
        for cell in &mut self.cell_rects {
            cell.rect = cell_rect(&self.rect, cell.id, self.mirrored);
        }

        self.intersection_rects = Location::all_intersections()
            .into_iter()
            .filter_map(|location| match location {
                Location::Intersection(locs, _) => {
                    intersection_rect(&self.rect, &locs, self.mirrored).map(|r| IntersectionRect {
                        locations: locs,
                        rect: r,
                    })
                }
                Location::Square(_, _) => None,
            })
            .collect();
    }

    fn region_sort_key(zone: &Zone) -> u8 {
        match zone {
            Zone::Location(Location::Square(_, region))
            | Zone::Location(Location::Intersection(_, region)) => match region {
                Region::Surface => 0,
                Region::Underground => 1,
                Region::Underwater => 2,
                Region::Void => 3,
            },
            _ => 4,
        }
    }

    fn location_choice_label(location: &Location) -> String {
        location.region().to_string()
    }

    fn sorted_location_choices(mut locations: Vec<Location>) -> Vec<Location> {
        locations.sort_by(|a, b| {
            Self::region_sort_key(&Zone::from(a))
                .cmp(&Self::region_sort_key(&Zone::from(b)))
                .then_with(|| a.to_string().cmp(&b.to_string()))
        });
        locations.dedup();
        locations
    }

    fn square_location_choices(locations: &[Location], cell_id: u8) -> Vec<Location> {
        Self::sorted_location_choices(
            locations
                .iter()
                .filter(|location| matches!(location, Location::Square(id, _) if *id == cell_id))
                .cloned()
                .collect(),
        )
    }

    fn intersection_location_choices(
        locations: &[Location],
        intersection_locations: &[u8],
    ) -> Vec<Location> {
        Self::sorted_location_choices(
            locations
                .iter()
                .filter(|location| {
                    matches!(
                        location,
                        Location::Intersection(locations, _)
                            if locations == intersection_locations
                    )
                })
                .cloned()
                .collect(),
        )
    }

    fn resolve_zone_choice(
        &mut self,
        location: &Location,
        action: PendingLocationChoiceAction,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        match action {
            PendingLocationChoiceAction::PickLocation => {
                self.client.send(ClientMessage::PickLocation {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    location: location.clone(),
                })?;
            }
            PendingLocationChoiceAction::PlayHandCard { card_id } => {
                self.client.send(ClientMessage::PlayCardAtLocation {
                    player_id: self.player_id,
                    game_id: self.game_id,
                    card_id,
                    location: location.clone(),
                })?;
            }
        }

        self.pending_zone_choice = None;
        data.status = Status::Idle;
        Ok(())
    }

    fn choose_zone_or_prompt(
        &mut self,
        locations: Vec<Location>,
        pos: Pos2,
        action: PendingLocationChoiceAction,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        match locations.as_slice() {
            [] => {}
            [location] => self.resolve_zone_choice(location, action, data)?,
            _ => {
                self.pending_zone_choice = Some(PendingLocationChoice {
                    pos,
                    locations,
                    action,
                });
            }
        }

        Ok(())
    }

    fn pending_zone_choice_is_valid(
        &self,
        choice: &PendingLocationChoice,
        data: &GameData,
    ) -> bool {
        match (&choice.action, &data.status) {
            (
                PendingLocationChoiceAction::PickLocation,
                Status::SelectingZone { locations, .. },
            ) => choice
                .locations
                .iter()
                .all(|location| locations.contains(location)),
            (
                PendingLocationChoiceAction::PlayHandCard { card_id },
                Status::PreviewingPlayableLocations {
                    card_id: preview_card_id,
                    locations,
                },
            ) => {
                card_id == preview_card_id
                    && choice
                        .locations
                        .iter()
                        .all(|location| locations.contains(location))
            }
            _ => false,
        }
    }

    fn render_zone_choice_picker(
        &mut self,
        ui: &mut Ui,
        data: &mut GameData,
    ) -> anyhow::Result<()> {
        let Some(choice) = self.pending_zone_choice.clone() else {
            return Ok(());
        };
        if !self.pending_zone_choice_is_valid(&choice, data) {
            self.pending_zone_choice = None;
            return Ok(());
        }

        let mut selected_location = None;
        egui::Area::new(egui::Id::new("realm_zone_choice_picker"))
            .fixed_pos(choice.pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for location in &choice.locations {
                            if ui
                                .button(Self::location_choice_label(location))
                                .on_hover_text(location.to_string())
                                .clicked()
                            {
                                selected_location = Some(location.clone());
                            }
                        }
                    });
                });
            });

        if let Some(location) = selected_location {
            self.resolve_zone_choice(&location, choice.action, data)?;
        }

        Ok(())
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
                Zone::Location(Location::Square(square, _)) => {
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
                Zone::Location(Location::Intersection(locs, _)) => {
                    if let Some(intersection) = self
                        .intersection_rects
                        .iter()
                        .find(|c| &c.locations == locs)
                    {
                        let existing = self.card_rects.iter().find(|c| c.card.id == card.id);
                        let rect = intersection.rect;
                        let dimensions = if card.card_type == CardType::Site {
                            site_dimensions(&rect)
                        } else {
                            projected_card_dimensions(&self.rect, locs[0], self.mirrored, false)
                        };

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
                if let Location::Square(id, _) = zone
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

    fn direction_vector(direction: &Direction) -> Vec2 {
        match direction {
            Direction::Up => vec2(0.0, -1.0),
            Direction::Down => vec2(0.0, 1.0),
            Direction::Left => vec2(-1.0, 0.0),
            Direction::Right => vec2(1.0, 0.0),
            Direction::TopLeft => vec2(-1.0, -1.0).normalized(),
            Direction::TopRight => vec2(1.0, -1.0).normalized(),
            Direction::BottomLeft => vec2(-1.0, 1.0).normalized(),
            Direction::BottomRight => vec2(1.0, 1.0).normalized(),
        }
    }

    fn location_center(&self, loc: &Location) -> Option<Pos2> {
        match loc {
            Location::Square(cell_id, _) => self
                .cell_rects
                .iter()
                .find(|cell| cell.id == *cell_id)
                .map(|cell| cell.rect.center()),
            Location::Intersection(locations, _) => self
                .intersection_rects
                .iter()
                .find(|intersection| intersection.locations == *locations)
                .map(|intersection| intersection.rect.center()),
        }
    }

    fn projectile_points(
        &self,
        data: &GameData,
        shooter: CardId,
        path: &[Location],
    ) -> Vec<Pos2> {
        let mut points = Vec::new();
        let fallback_start = data
            .cards
            .iter()
            .find(|card| card.id == shooter)
            .and_then(|card| self.location_center(&card.zone.location().cloned().unwrap()));
        for location in path {
            if let Some(point) = self.location_center(location) {
                points.push(point);
            } else if points.is_empty()
                && let Some(fallback_start) = fallback_start
            {
                points.push(fallback_start);
            }
        }

        points
    }

    fn start_pending_projectiles(&mut self, data: &mut GameData, ctx: &Context) {
        let started_at = ctx.input(|i| i.time);
        let pending = std::mem::take(&mut data.pending_projectiles);
        for projectile in pending {
            if self
                .projectile_flights
                .iter()
                .any(|flight| flight.id == projectile.id)
            {
                continue;
            }

            let points = self.projectile_points(
                data,
                projectile.shooter,
                &projectile.path,
            );
            if points.len() < 2 {
                continue;
            }

            self.projectile_flights.push(ProjectileFlight {
                id: projectile.id,
                points,
                direction: projectile.direction,
                started_at,
                ranged_strike: projectile.ranged_strike,
            });
        }
    }

    fn sample_projectile_path(points: &[Pos2], progress: f32) -> (Pos2, Vec2) {
        let total_len = points
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).length())
            .sum::<f32>()
            .max(1.0);
        let mut target_len = total_len * progress.clamp(0.0, 1.0);

        for pair in points.windows(2) {
            let segment = pair[1] - pair[0];
            let len = segment.length();
            if target_len <= len {
                let t = if len > 0.0 { target_len / len } else { 0.0 };
                return (pair[0] + segment * t, segment.normalized());
            }
            target_len -= len;
        }

        let last = *points.last().unwrap_or(&pos2(0.0, 0.0));
        let prev = points
            .iter()
            .rev()
            .nth(1)
            .copied()
            .unwrap_or(last - vec2(0.0, 1.0));
        (last, (last - prev).normalized())
    }

    fn render_projectile_flights(&mut self, ui: &mut Ui, painter: &Painter, now: f64) {
        self.projectile_flights.retain(|flight| {
            let progress =
                ((now - flight.started_at) / PROJECTILE_FLIGHT_DURATION).clamp(0.0, 1.0) as f32;
            let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
            let (pos, dir) = Self::sample_projectile_path(&flight.points, eased);
            let dir = if dir.length_sq() > 0.0 {
                dir
            } else {
                Self::direction_vector(&flight.direction)
            };

            let color = if flight.ranged_strike {
                Color32::from_rgb(255, 224, 92)
            } else {
                Color32::from_rgb(98, 190, 255)
            };
            let core = if flight.ranged_strike {
                Color32::from_rgb(255, 255, 210)
            } else {
                Color32::from_rgb(220, 245, 255)
            };
            let tail = pos - dir * 42.0;
            painter.line_segment(
                [tail, pos - dir * 8.0],
                Stroke::new(
                    5.0,
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 56),
                ),
            );
            painter.line_segment(
                [tail + dir * 12.0, pos - dir * 5.0],
                Stroke::new(
                    2.0,
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 190),
                ),
            );
            painter.circle_filled(
                pos,
                8.0,
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 85),
            );
            painter.circle_filled(pos, 4.0, core);

            if progress < 1.0 {
                ui.ctx().request_repaint();
                true
            } else {
                false
            }
        });
    }

    fn direction_origin(&self, data: &GameData, source_card_id: Option<CardId>) -> Pos2 {
        if let Some(source_card_id) = source_card_id {
            if let Some(card_rect) = self
                .card_rects
                .iter()
                .find(|card_rect| card_rect.card.id == source_card_id)
            {
                return card_rect.rect.center();
            }

            if let Some(card) = data.cards.iter().find(|card| card.id == source_card_id) {
                match &card.zone {
                    Zone::Location(Location::Square(cell_id, _)) => {
                        if let Some(cell) = self.cell_rects.iter().find(|cell| cell.id == *cell_id)
                        {
                            return cell.rect.center();
                        }
                    }
                    Zone::Location(Location::Intersection(locations, _)) => {
                        if let Some(intersection) = self
                            .intersection_rects
                            .iter()
                            .find(|intersection| &intersection.locations == locations)
                        {
                            return intersection.rect.center();
                        }
                    }
                    _ => {}
                }
            }
        }

        let corners = board_corners(&self.rect);
        pos2(
            corners.iter().map(|point| point.x).sum::<f32>() / corners.len() as f32,
            corners.iter().map(|point| point.y).sum::<f32>() / corners.len() as f32,
        )
    }

    fn clamp_direction_origin(
        origin: Pos2,
        directions: &[Direction],
        radius: f32,
        button_size: f32,
        bounds: Rect,
    ) -> Pos2 {
        if directions.is_empty() {
            return origin;
        }

        let margin = button_size * 0.5 + 8.0;
        let mut min_offset = vec2(f32::INFINITY, f32::INFINITY);
        let mut max_offset = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for direction in directions {
            let offset = Self::direction_vector(direction) * radius;
            min_offset.x = min_offset.x.min(offset.x - margin);
            min_offset.y = min_offset.y.min(offset.y - margin);
            max_offset.x = max_offset.x.max(offset.x + margin);
            max_offset.y = max_offset.y.max(offset.y + margin);
        }

        let clamp_axis =
            |value: f32, min_bound: f32, max_bound: f32, min_offset: f32, max_offset: f32| {
                let min_value = min_bound - min_offset;
                let max_value = max_bound - max_offset;
                if min_value <= max_value {
                    value.clamp(min_value, max_value)
                } else {
                    (min_value + max_value) * 0.5
                }
            };

        pos2(
            clamp_axis(
                origin.x,
                bounds.min.x,
                bounds.max.x,
                min_offset.x,
                max_offset.x,
            ),
            clamp_axis(
                origin.y,
                bounds.min.y,
                bounds.max.y,
                min_offset.y,
                max_offset.y,
            ),
        )
    }

    fn render_direction_picker(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut GameData,
        painter: &Painter,
    ) -> anyhow::Result<()> {
        let Status::SelectingDirection {
            directions,
            source_card_id,
            ..
        } = &data.status.clone()
        else {
            return Ok(());
        };

        let raw_origin = self.direction_origin(data, *source_card_id);
        let board = Rect::from_points(&board_corners(&self.rect));
        let radius = (board.width().min(board.height()) * 0.24).clamp(58.0, 128.0);
        let button_size = (board.width().min(board.height()) * 0.11).clamp(42.0, 58.0);
        let origin = Self::clamp_direction_origin(
            raw_origin,
            directions,
            radius,
            button_size,
            ui.clip_rect(),
        );
        let mut picked = None;

        painter.circle_stroke(
            origin,
            radius,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(220, 235, 255, 58)),
        );

        for direction in directions {
            let vector = Self::direction_vector(direction);
            let center = origin + vector * radius;
            let rect = Rect::from_center_size(center, vec2(button_size, button_size));
            let response = ui.interact(
                rect,
                ui.id().with(format!("direction_picker_{direction:?}")),
                Sense::click(),
            );
            let hovered = response.hovered();
            let fill = if hovered {
                theme::ACTION_HOVERED
            } else {
                Color32::from_rgba_unmultiplied(32, 50, 70, 230)
            };
            let stroke = if hovered {
                Stroke::new(2.0, theme::PICKABLE)
            } else {
                Stroke::new(1.0, theme::PANEL_BORDER)
            };

            painter.circle_filled(center, button_size * 0.5, fill);
            painter.circle_stroke(center, button_size * 0.5, stroke);
            let shaft_start = center - vector * (button_size * 0.18);
            let shaft_end = center + vector * (button_size * 0.18);
            painter.line_segment(
                [shaft_start, shaft_end],
                Stroke::new(3.0, theme::TEXT_BRIGHT),
            );
            let perp = vec2(-vector.y, vector.x);
            let tip = center + vector * (button_size * 0.28);
            let head_left = tip - vector * (button_size * 0.18) + perp * (button_size * 0.12);
            let head_right = tip - vector * (button_size * 0.18) - perp * (button_size * 0.12);
            painter.add(Shape::convex_polygon(
                vec![tip, head_left, head_right],
                theme::TEXT_BRIGHT,
                Stroke::NONE,
            ));

            if response.clicked() {
                picked = Some(direction.clone());
            }
            if hovered {
                ui.set_cursor_icon(CursorIcon::PointingHand);
            }
        }

        if let Some(direction) = picked {
            self.client.send(ClientMessage::PickDirection {
                player_id: self.player_id,
                game_id: self.game_id,
                direction,
            })?;
            data.status = Status::Idle;
        }

        Ok(())
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

    fn draw_affected_zone_highlight(
        painter: &Painter,
        zones: &[Zone],
        realm_rect: Rect,
        mirrored: bool,
        intersection_rects: &[IntersectionRect],
    ) {
        let fill = Color32::from_rgba_unmultiplied(80, 200, 190, 62);
        let stroke = Stroke::new(2.5, Color32::from_rgba_unmultiplied(120, 235, 220, 210));

        for zone in zones {
            match zone {
                Zone::Location(Location::Square(cell_id, _)) => {
                    painter.add(Shape::convex_polygon(
                        cell_corners(&realm_rect, *cell_id, mirrored, 5.0).to_vec(),
                        fill,
                        stroke,
                    ));
                }
                Zone::Location(Location::Intersection(locations, _)) => {
                    if let Some(intersection) = intersection_rects
                        .iter()
                        .find(|intersection| &intersection.locations == locations)
                    {
                        painter.rect_filled(intersection.rect, 4.0, fill);
                        painter.rect_stroke(
                            intersection.rect,
                            4.0,
                            stroke,
                            egui::StrokeKind::Outside,
                        );
                    }
                }
                _ => {}
            }
        }
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
                Zone::Location(Location::Square(loc, _)) => Some(*loc),
                _ => None,
            })
            .collect();

        self.draw_playmat(painter);

        let mut clicked_zone_choices = None;
        for cell in &self.cell_rects {
            let rect = cell.rect;

            self.draw_zone_guide(painter, cell.id, occupied_zones.contains(&cell.id));

            let playable_preview_zones = match &data.status {
                Status::SelectingZone { locations, .. }
                | Status::PreviewingPlayableLocations { locations, .. } => Some(locations),
                _ => None,
            };
            if let Some(locations) = playable_preview_zones {
                let choices = Self::square_location_choices(locations, cell.id);
                if !choices.is_empty() {
                    if matches!(data.status, Status::SelectingZone { .. }) {
                        let resp = ui.allocate_rect(rect, Sense::click());
                        if resp.clicked() {
                            clicked_zone_choices = Some((choices.clone(), rect.center()));
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
                    | Status::GameOver { .. }
                    | Status::SelectingAction { .. }
                    | Status::SelectingDirection { .. }
                    | Status::Waiting { .. }
                    | Status::SelectingPath { .. }
                    | Status::SelectingAmount { .. }
                    | Status::Mulligan
                    | Status::ViewingCards { .. } => {
                        continue;
                    }
                    Status::SelectingCard { preview: false, .. } | Status::Idle => {}
                    Status::SelectingZone { .. } | Status::PreviewingPlayableLocations { .. } => {}
                }
            }
        }

        for intersection in &self.intersection_rects {
            let playable_preview_zones = match &data.status {
                Status::SelectingZone { locations, .. }
                | Status::PreviewingPlayableLocations { locations, .. } => Some(locations),
                _ => None,
            };
            if let Some(locations) = playable_preview_zones {
                let rect = intersection.rect;
                let choices =
                    Self::intersection_location_choices(locations, &intersection.locations);
                if !choices.is_empty() {
                    if matches!(data.status, Status::SelectingZone { .. }) {
                        let resp = ui.allocate_rect(rect, Sense::click());
                        if resp.clicked() {
                            clicked_zone_choices = Some((choices.clone(), rect.center()));
                        }
                    }

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
                    | Status::SelectingDirection { .. }
                    | Status::SelectingAmount { .. }
                    | Status::Mulligan
                    | Status::GameAborted { .. }
                    | Status::GameOver { .. }
                    | Status::SelectingPath { .. }
                    | Status::ViewingCards { .. } => {
                        continue;
                    }
                    Status::SelectingCard { preview: false, .. } | Status::Idle => {}
                    Status::SelectingZone { .. } | Status::PreviewingPlayableLocations { .. } => {}
                }
            }
        }

        if let Some((locations, pos)) = clicked_zone_choices {
            self.choose_zone_or_prompt(
                locations,
                pos,
                PendingLocationChoiceAction::PickLocation,
                data,
            )?;
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
                        .any(|zone| matches!(zone, Location::Square(id, _) if *id == cell_rect.id))
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
                    if let Location::Square(cell_id, _) = zone
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

    fn render_view_controls(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
    ) -> anyhow::Result<Option<OngoingEffectData>> {
        let mut hovered_effect = None;
        let button_pos = pos2(self.rect.min.x + 18.0, self.rect.min.y + 18.0);
        egui::Area::new(egui::Id::new("realm_view_controls"))
            .fixed_pos(button_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
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
                    ui.separator();
                    let effects_clicked = ui
                        .selectable_label(data.show_ongoing_effects, "Effects")
                        .on_hover_text("Show active ongoing effects")
                        .clicked();
                    if effects_clicked {
                        data.show_ongoing_effects = !data.show_ongoing_effects;
                        if data.show_ongoing_effects {
                            data.ongoing_effects = None;
                        }
                    }
                });
            });
        if data.show_ongoing_effects {
            egui::Window::new("Ongoing effects")
                .id(egui::Id::new("ongoing_effects_window"))
                .order(egui::Order::Tooltip)
                .default_pos(pos2(self.rect.min.x + 18.0, self.rect.min.y + 58.0))
                .default_width(340.0)
                .resizable(true)
                .collapsible(false)
                .open(&mut data.show_ongoing_effects)
                .show(ui.ctx(), |ui| {
                    if data.ongoing_effects.is_none() {
                        ui.label("Loading effects...");
                    } else if let Some(effects) = &data.ongoing_effects {
                        egui::ScrollArea::vertical()
                            .max_height(260.0)
                            .show(ui, |ui| {
                                if effects.is_empty() {
                                    ui.label("No ongoing effects");
                                }
                                for effect in effects {
                                    let source =
                                        effect.source_name.as_deref().unwrap_or("No source");
                                    let title = format!("{}: {}", source, effect.description);
                                    let text = if effect.active {
                                        egui::RichText::new(title)
                                    } else {
                                        egui::RichText::new(format!("{} (inactive)", title))
                                            .color(theme::TURN_WAITING)
                                    };
                                    let response = ui.label(text).on_hover_text(
                                        "Hover to highlight affected realm zones and cards",
                                    );
                                    if response.hovered() {
                                        hovered_effect = Some(effect.clone());
                                    }
                                }
                            });
                    }
                });
        }
        if data.show_ongoing_effects && data.ongoing_effects.is_none() {
            self.client.send(ClientMessage::RequestOngoingEffects {
                player_id: self.player_id,
                game_id: self.game_id,
            })?;
        }
        Ok(hovered_effect)
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

    fn card_clicked(&mut self, card_id: &CardId, data: &mut GameData) -> anyhow::Result<()> {
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

            self.client.send(ClientMessage::PickLocationGroup {
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
}

impl Component for RealmComponent {
    fn update(&mut self, data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        self.refresh_geometry();
        self.compute_rects(&data.cards, ctx)?;
        self.start_pending_projectiles(data, ctx);

        // If a card was clicked within the last 3 seconds, find it and animate it flying to its new
        // position.
        if let Some(card_id) = data.last_clicked_card_id {
            let click_age = data
                .last_clicked_card_time
                .map(|click_time| ctx.input(|i| i.time) - click_time)
                .unwrap_or(0.0);

            if click_age > 3.0 {
                data.last_clicked_card_id = None;
                data.last_clicked_card_rect = None;
                data.last_clicked_cursor_pos = None;
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
                data.last_clicked_card_rect = None;
                data.last_clicked_cursor_pos = None;
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

        let highlighted_effect = data.highlighted_ongoing_effect.clone();
        let mut clicked_card = None;
        let mut move_delta = Vec2::default();
        let mut moved_card_id = None;
        let card_clicks_enabled = matches!(
            data.status,
            Status::Idle | Status::SelectingCard { preview: false, .. }
        );
        let suppress_preview = matches!(
            data.status,
            Status::SelectingAction { .. }
                | Status::SelectingDirection { .. }
                | Status::SelectingPath { .. }
                | Status::SelectingZone { .. }
                | Status::SelectingZoneGroup { .. }
                | Status::SelectingAmount { .. }
                | Status::Waiting { .. }
                | Status::GameAborted { .. }
                | Status::GameOver { .. }
        );
        let realm_rect = self.rect;
        let mirrored = self.mirrored;
        let card_filter = self.card_filter;
        let intersection_rects = self.intersection_rects.clone();

        if let Some(effect) = &highlighted_effect {
            Self::draw_affected_zone_highlight(
                painter,
                &effect.affected_zones,
                realm_rect,
                mirrored,
                &intersection_rects,
            );
        }

        for card_rect in &mut self.card_rects {
            if !card_rect.card.zone.is_in_play() {
                continue;
            }

            let pickable_in_current_prompt = matches!(
                &data.status,
                Status::SelectingCard {
                    cards,
                    preview: false,
                    ..
                } if cards.contains(&card_rect.card.id)
            );
            if !card_filter.includes(&card_rect.card) && !pickable_in_current_prompt {
                continue;
            }

            if self
                .card_flights
                .iter()
                .any(|flight| flight.card_id == card_rect.card.id)
            {
                continue;
            }

            let sense = if card_clicks_enabled {
                Sense::HOVER | Sense::CLICK | Sense::DRAG
            } else {
                Sense::HOVER
            };
            let resp = ui.allocate_rect(card_rect.rect, sense);
            if resp.hovered() && card_clicks_enabled && card_rect.card.card_type == CardType::Aura {
                match data.aura_areas_of_effect.get(&card_rect.card.id) {
                    Some(Some(locations)) if !locations.is_empty() => {
                        Self::draw_affected_zone_highlight(
                            painter,
                            &locations
                                .iter()
                                .map(|l| l.clone().into())
                                .collect::<Vec<_>>(),
                            realm_rect,
                            mirrored,
                            &intersection_rects,
                        );
                    }
                    Some(_) => {}
                    None => {
                        // Do not request aura affected zones if waiting for any other message.
                        if data.status == Status::Idle {
                            data.aura_areas_of_effect.insert(card_rect.card.id, None);
                            self.client.send(ClientMessage::RequestAuraAreaOfEffect {
                                player_id: self.player_id,
                                game_id: self.game_id,
                                card_id: card_rect.card.id,
                            })?;
                        }
                    }
                }
            }

            if matches!(card_rect.card.zone, Zone::Location(Location::Square(_, _))) {
                let corners = card_corners(card_rect.rect, card_rotation(&card_rect.card));
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

            let visual_rect =
                if matches!(card_rect.card.zone, Zone::Location(Location::Square(_, _))) {
                    Rect::from_points(&card_corners(
                        card_rect.rect,
                        card_rotation(&card_rect.card),
                    ))
                } else {
                    card_rect.rect
                };

            if highlighted_effect
                .as_ref()
                .is_some_and(|effect| effect.affected_card_ids.contains(&card_rect.card.id))
            {
                let stroke = Stroke::new(3.0, Color32::from_rgba_unmultiplied(120, 235, 220, 230));
                if matches!(card_rect.card.zone, Zone::Location(Location::Square(_, _))) {
                    painter.add(Shape::closed_line(
                        card_corners(card_rect.rect, card_rotation(&card_rect.card)).to_vec(),
                        stroke,
                    ));
                } else {
                    painter.rect_stroke(visual_rect, 4.0, stroke, egui::StrokeKind::Outside);
                }
            }

            if card_clicks_enabled && resp.clicked() {
                let click_pos = resp.interact_pointer_pos().unwrap_or(visual_rect.center());
                clicked_card = Some((card_rect.card.id, visual_rect, click_pos));
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

            if resp.hovered() && !resp.clicked() && !suppress_preview {
                render::draw_card_preview(ui, card_rect.image.as_ref())?;
            }

            let cell = match &card_rect.card.zone {
                Zone::Location(Location::Square(cell_id, _)) => {
                    self.cell_rects.iter().find(|c| c.id == *cell_id)
                }
                _ => None,
            };

            if card_clicks_enabled
                && resp.dragged()
                && let Some(cell) = cell
            {
                let card_id = card_rect.card.id;
                let bounds = match card_rect.card.zone {
                    Zone::Location(Location::Square(cell_id, _)) => {
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
                if matches!(card_rect.card.zone, Zone::Location(Location::Square(_, _))) {
                    painter.add(Shape::convex_polygon(
                        card_corners(card_rect.rect, card_rotation(&card_rect.card)).to_vec(),
                        overlay_color,
                        Stroke::NONE,
                    ));
                } else {
                    painter.rect_filled(card_rect.rect, 0.0, overlay_color);
                }
            }
        }

        if let Some(card_id) = moved_card_id {
            let attached_cards: Vec<CardId> = self
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

        if let Some((card_id, card_rect, click_pos)) = clicked_card {
            if matches!(data.status, Status::Idle) {
                data.last_clicked_card_pos = Some(card_rect.center());
                data.last_clicked_card_rect = Some(card_rect);
                data.last_clicked_cursor_pos = Some(click_pos);
            }
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
            if matches!(card_rect.card.zone, Zone::Location(Location::Square(_, _))) {
                render::draw_projected_card(
                    &card_rect,
                    card_rect.card.controller_id == self.player_id,
                    true,
                    painter,
                    card_corners(card_rect.rect, rotation),
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
        self.render_projectile_flights(ui, painter, now);

        self.render_paths(ui, data, painter);
        self.render_direction_picker(ui, data, painter)?;
        data.highlighted_ongoing_effect = self.render_view_controls(data, ui)?;
        self.render_zone_choice_picker(ui, data)?;

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
                if let Status::PreviewingPlayableLocations {
                    card_id: preview_card_id,
                    locations,
                } = &data.status.clone()
                    && preview_card_id == card_id
                {
                    let dropped_zone_choices = self
                        .cell_rects
                        .iter()
                        .find(|cell| cell.rect.contains(*pos))
                        .map(|cell| {
                            (
                                Self::square_location_choices(locations, cell.id),
                                cell.rect.center(),
                            )
                        })
                        .or_else(|| {
                            self.intersection_rects
                                .iter()
                                .find(|intersection| intersection.rect.contains(*pos))
                                .map(|intersection| {
                                    (
                                        Self::intersection_location_choices(
                                            locations,
                                            &intersection.locations,
                                        ),
                                        intersection.rect.center(),
                                    )
                                })
                        });

                    if let Some((locations, pos)) = dropped_zone_choices {
                        self.choose_zone_or_prompt(
                            locations,
                            pos,
                            PendingLocationChoiceAction::PlayHandCard { card_id: *card_id },
                            data,
                        )?;
                    } else {
                        data.status = Status::Idle;
                    }
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
