use egui::epaint::{CornerRadius, Shape};
use egui::{
    Align2, Color32, Context, FontId, Painter, Rect, Response, Stroke, TextureHandle, Ui, pos2,
    vec2,
};
use sorcerers::{
    game::PlayerId,
    game::{Element, Resources, Thresholds},
    zone::Zone,
};

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    element_icon,
    scene::game::GameData,
    theme,
};

// ── Layout ─────────────────────────────────────────────────────────────────────
const THRESH_SYM: f32 = 13.0; // triangle bounding box size
const PAD_H: i8 = 8; // horizontal inner margin (i8 for egui::Margin)
const PAD_V: i8 = 6; // vertical   inner margin
const DEATHS_DOOR_MEDALLION: &[u8] =
    include_bytes!("../../../../assets/images/hud/deaths_door_reaper_v3.png");

// Shared side rail provides the panel background; the status widgets retain its border color.
const BORDER: Color32 = theme::PANEL_BORDER;

pub struct PlayerStatusComponent {
    visible: bool,
    player_id: PlayerId,
    /// `true` = local player (left rail), `false` = opponent (right rail).
    player: bool,
    rect: Rect,
    deaths_door_texture: Option<TextureHandle>,
}

impl std::fmt::Debug for PlayerStatusComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerStatusComponent")
            .field("visible", &self.visible)
            .field("player_id", &self.player_id)
            .field("player", &self.player)
            .field("rect", &self.rect)
            .finish()
    }
}

impl PlayerStatusComponent {
    pub fn new(rect: Rect, player_id: PlayerId, player: bool) -> Self {
        Self {
            visible: true,
            player_id,
            rect,
            player,
            deaths_door_texture: None,
        }
    }
}

/// Official Sorcery: Contested Realm element symbol.
///
/// Fire  = upward triangle               (▲)
/// Air   = upward triangle + horizontal midline
/// Earth = downward triangle + horizontal midline
/// Water = downward triangle              (▽)
fn threshold_cell(painter: &Painter, center: egui::Pos2, count: u8, element: Element) {
    element_icon::draw_element_triangle(
        painter,
        &element,
        center + vec2(-6.0, 0.0),
        THRESH_SYM,
        1.5,
    );
    painter.text(
        center + vec2(8.0, 1.5),
        Align2::CENTER_CENTER,
        count.to_string(),
        FontId::proportional(THRESH_SYM * 0.85),
        element_icon::element_color(&element),
    );
}

fn threshold_grid(ui: &mut Ui, thresholds: Thresholds) {
    let (rect, _) = ui.allocate_exact_size(vec2(78.0, 34.0), egui::Sense::hover());
    let painter = ui.painter();
    let left = rect.center().x - 18.0;
    let right = rect.center().x + 18.0;
    let top = rect.min.y + 9.0;
    let bottom = rect.min.y + 25.0;
    threshold_cell(painter, pos2(left, top), thresholds.fire, Element::Fire);
    threshold_cell(painter, pos2(right, top), thresholds.air, Element::Air);
    threshold_cell(
        painter,
        pos2(left, bottom),
        thresholds.earth,
        Element::Earth,
    );
    threshold_cell(
        painter,
        pos2(right, bottom),
        thresholds.water,
        Element::Water,
    );
}

#[derive(Clone, Copy)]
enum StatIcon {
    Mana,
    Hand,
    Cemetery,
    Banish,
}

fn status_icon(painter: &Painter, rect: Rect, icon: StatIcon, color: Color32) {
    let center = rect.center();
    let stroke = Stroke::new(1.5, color);
    match icon {
        StatIcon::Mana => {
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(center.x, rect.min.y),
                    pos2(rect.max.x, center.y),
                    pos2(center.x, rect.max.y),
                    pos2(rect.min.x, center.y),
                ],
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 48),
                stroke,
            ));
        }
        StatIcon::Hand => {
            let back = rect.translate(vec2(-2.0, -2.0)).shrink(2.0);
            painter.rect_stroke(back, 2.0, stroke, egui::StrokeKind::Inside);
            painter.rect_stroke(rect.shrink(2.0), 2.0, stroke, egui::StrokeKind::Inside);
        }
        StatIcon::Cemetery => {
            let stone = Rect::from_min_max(
                pos2(rect.min.x + 3.0, center.y - 1.0),
                pos2(rect.max.x - 3.0, rect.max.y - 1.0),
            );
            painter.rect_filled(
                stone,
                CornerRadius::same(4),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 36),
            );
            painter.rect_stroke(stone, 4.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    pos2(center.x, stone.min.y + 3.0),
                    pos2(center.x, stone.max.y - 3.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center.x - 3.0, center.y + 2.0),
                    pos2(center.x + 3.0, center.y + 2.0),
                ],
                stroke,
            );
        }
        StatIcon::Banish => {
            painter.circle_stroke(center, rect.width() * 0.38, stroke);
            painter.add(Shape::closed_line(
                vec![
                    pos2(center.x, rect.min.y + 2.0),
                    pos2(rect.max.x - 2.0, center.y),
                    pos2(center.x, rect.max.y - 2.0),
                    pos2(rect.min.x + 2.0, center.y),
                ],
                stroke,
            ));
        }
    }
}

fn side_stat(
    ui: &mut Ui,
    icon: StatIcon,
    label: &str,
    value: impl std::fmt::Display,
    color: Color32,
    id: &'static str,
    clickable: bool,
) -> Response {
    let response = ui
        .push_id(id, |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(82.0, 25.0), egui::Sense::click());
            let hovered = clickable && response.hovered();
            let fill = if hovered {
                Color32::from_rgba_unmultiplied(60, 78, 96, 230)
            } else {
                Color32::from_rgba_unmultiplied(30, 40, 52, 190)
            };
            ui.painter().rect_filled(rect, CornerRadius::same(5), fill);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0, if hovered { theme::PICKABLE } else { BORDER }),
                egui::StrokeKind::Outside,
            );
            let icon_rect =
                Rect::from_center_size(pos2(rect.min.x + 14.0, rect.center().y), vec2(16.0, 16.0));
            status_icon(ui.painter(), icon_rect, icon, color);
            ui.painter().text(
                pos2(rect.min.x + 27.0, rect.min.y + 5.0),
                Align2::LEFT_TOP,
                label,
                FontId::proportional(7.0),
                theme::TURN_WAITING,
            );
            let value_center = pos2(rect.max.x - 11.0, rect.center().y);
            ui.painter().text(
                value_center + vec2(0.0, 1.5),
                Align2::CENTER_CENTER,
                value.to_string(),
                FontId::proportional(11.0),
                color,
            );
            response
        })
        .inner;

    if clickable && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response
}

fn draw_seat_housing(painter: &Painter, rect: Rect, is_self: bool) {
    let edge = if is_self {
        Color32::from_rgb(66, 118, 142)
    } else {
        Color32::from_rgb(128, 75, 62)
    };
    let outer = rect.expand(3.0);
    painter.rect_filled(
        outer,
        CornerRadius::same(9),
        Color32::from_rgba_unmultiplied(5, 8, 10, 244),
    );
    painter.rect_stroke(
        outer,
        CornerRadius::same(9),
        Stroke::new(
            1.5,
            Color32::from_rgba_unmultiplied(edge.r(), edge.g(), edge.b(), 190),
        ),
        egui::StrokeKind::Outside,
    );
    painter.rect_filled(
        rect,
        CornerRadius::same(7),
        Color32::from_rgba_unmultiplied(15, 20, 22, 242),
    );
    painter.rect_stroke(
        rect.shrink(4.0),
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(190, 202, 190, 40)),
        egui::StrokeKind::Inside,
    );
    let highlight = Color32::from_rgba_unmultiplied(222, 231, 218, 54);
    let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, 170);
    painter.line_segment(
        [
            pos2(rect.min.x + 7.0, rect.min.y + 3.0),
            pos2(rect.max.x - 7.0, rect.min.y + 3.0),
        ],
        Stroke::new(1.0, highlight),
    );
    painter.line_segment(
        [
            pos2(rect.min.x + 3.0, rect.min.y + 7.0),
            pos2(rect.min.x + 3.0, rect.max.y - 7.0),
        ],
        Stroke::new(1.0, highlight),
    );
    painter.line_segment(
        [
            pos2(rect.min.x + 7.0, rect.max.y - 3.0),
            pos2(rect.max.x - 7.0, rect.max.y - 3.0),
        ],
        Stroke::new(1.5, shadow),
    );
    painter.line_segment(
        [
            pos2(rect.max.x - 3.0, rect.min.y + 7.0),
            pos2(rect.max.x - 3.0, rect.max.y - 7.0),
        ],
        Stroke::new(1.5, shadow),
    );
    for corner in [
        pos2(rect.min.x + 8.0, rect.min.y + 8.0),
        pos2(rect.max.x - 8.0, rect.min.y + 8.0),
        pos2(rect.min.x + 8.0, rect.max.y - 8.0),
        pos2(rect.max.x - 8.0, rect.max.y - 8.0),
    ] {
        painter.circle_filled(corner, 2.5, Color32::from_rgb(20, 26, 28));
        painter.circle_stroke(
            corner,
            2.5,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(189, 169, 112, 120)),
        );
        painter.circle_filled(
            corner + vec2(-0.6, -0.6),
            0.8,
            Color32::from_rgb(205, 207, 188),
        );
    }
}

fn liquid_segment(
    painter: &Painter,
    center: egui::Pos2,
    radius: f32,
    surface_y: f32,
    color: Color32,
) {
    if surface_y >= center.y + radius {
        return;
    }
    if surface_y <= center.y - radius {
        painter.circle_filled(center, radius, color);
        return;
    }

    let normalized = ((surface_y - center.y) / radius).clamp(-1.0, 1.0);
    let start = normalized.asin();
    let end = std::f32::consts::PI - start;
    let mut points = Vec::with_capacity(26);
    for step in 0..=24 {
        let angle = start + (end - start) * step as f32 / 24.0;
        points.push(center + vec2(angle.cos() * radius, angle.sin() * radius));
    }
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn life_vial(ui: &mut Ui, health: u16, deaths_door: bool, texture: Option<&TextureHandle>) {
    let max_health = 20.0;
    let fill_fraction = if deaths_door {
        0.05
    } else {
        (health as f32 / max_health).clamp(0.0, 1.0)
    };
    let (rect, response) = ui.allocate_exact_size(vec2(82.0, 84.0), egui::Sense::hover());
    let painter = ui.painter();
    let center = pos2(rect.center().x, rect.center().y + 3.0);
    let rim = if deaths_door {
        Color32::from_rgb(126, 104, 70)
    } else {
        Color32::from_rgb(147, 54, 61)
    };
    painter.circle_filled(center, 40.0, Color32::from_rgb(7, 9, 10));
    painter.circle_stroke(
        center,
        39.0,
        Stroke::new(2.0, Color32::from_rgb(78, 82, 74)),
    );
    painter.circle_stroke(center, 36.0, Stroke::new(1.5, rim));
    if deaths_door {
        if let Some(texture) = texture {
            painter.image(
                texture.id(),
                Rect::from_center_size(center, vec2(70.0, 70.0)),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::from_rgb(226, 226, 218),
            );
            painter.circle_stroke(
                center,
                34.0,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(186, 76, 58, 90)),
            );
        }
        return;
    }
    let liquid_radius = 33.0;
    let surface_y = center.y + liquid_radius - liquid_radius * 2.0 * fill_fraction;
    painter.circle_filled(center, liquid_radius, Color32::from_rgb(18, 12, 15));
    for (radius, color) in [
        (33.0, Color32::from_rgb(55, 13, 18)),
        (29.0, Color32::from_rgb(82, 18, 24)),
        (24.0, Color32::from_rgb(108, 23, 30)),
        (18.0, Color32::from_rgb(132, 29, 37)),
    ] {
        liquid_segment(painter, center, radius, surface_y, color);
    }
    if fill_fraction > 0.0 && fill_fraction < 1.0 {
        let half_width = (liquid_radius * liquid_radius
            - (surface_y - center.y) * (surface_y - center.y))
            .max(0.0)
            .sqrt();
        painter.line_segment(
            [
                pos2(center.x - half_width, surface_y),
                pos2(center.x + half_width, surface_y),
            ],
            Stroke::new(1.2, Color32::from_rgba_unmultiplied(222, 76, 75, 175)),
        );
    }
    // The reflection belongs to the glass, so it remains visible regardless of
    // how much liquid is left in the orb.
    painter.circle_filled(
        center + vec2(-11.0, -13.0),
        10.0,
        Color32::from_rgba_unmultiplied(232, 239, 238, 25),
    );
    painter.circle_filled(
        center + vec2(-14.0, -16.0),
        3.5,
        Color32::from_rgba_unmultiplied(255, 255, 246, 105),
    );
    let reflection_arc: Vec<egui::Pos2> = (0..=12)
        .map(|step| {
            let angle = 3.55 + (4.42 - 3.55) * step as f32 / 12.0;
            center + vec2(angle.cos() * 30.5, angle.sin() * 30.5)
        })
        .collect();
    painter.add(Shape::line(
        reflection_arc,
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(238, 244, 238, 90)),
    ));
    painter.circle_stroke(
        center,
        33.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 206, 192, 45)),
    );
    if response.hovered() {
        painter.text(
            center + vec2(0.0, 3.0),
            Align2::CENTER_CENTER,
            health.to_string(),
            FontId::proportional(18.0),
            Color32::WHITE,
        );
    }
}

fn seat_crest(ui: &mut Ui, is_self: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(78.0, 14.0), egui::Sense::hover());
    let accent = if is_self {
        Color32::from_rgb(90, 176, 218)
    } else {
        Color32::from_rgb(190, 112, 86)
    };
    let painter = ui.painter();
    let center = rect.center();
    let gap = 8.0;
    let line = Stroke::new(
        1.0,
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 145),
    );
    painter.line_segment(
        [
            pos2(rect.min.x + 2.0, center.y),
            pos2(center.x - gap, center.y),
        ],
        line,
    );
    painter.line_segment(
        [
            pos2(center.x + gap, center.y),
            pos2(rect.max.x - 2.0, center.y),
        ],
        line,
    );
    painter.add(Shape::convex_polygon(
        vec![
            pos2(center.x, center.y - 4.0),
            pos2(center.x + 4.0, center.y),
            pos2(center.x, center.y + 4.0),
            pos2(center.x - 4.0, center.y),
        ],
        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 52),
        line,
    ));
}

impl Component for PlayerStatusComponent {
    fn update(&mut self, _data: &mut GameData, _ctx: &Context) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        _painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if !self.visible {
            return Ok(None);
        }

        let sr = crate::config::screen_rect()?;
        let ctx = ui.ctx().clone();
        let panel_w = 98.0;
        let top_margin = 72.0;
        let bottom_margin = 96.0;
        let panel_h = 316.0;
        let panel_y = if self.player {
            sr.max.y - bottom_margin - panel_h
        } else {
            top_margin
        };
        // The fixed player rail spans x=12..128 (116px). Center this 98px HUD
        // within it rather than inheriting the rail's left edge.
        let panel_x = 21.0;
        let panel_pos = pos2(panel_x, panel_y);
        let panel_rect = Rect::from_min_size(panel_pos, vec2(panel_w, panel_h));
        self.rect = panel_rect;

        // Gather data
        let resources = data
            .resources
            .get(&self.player_id)
            .cloned()
            .unwrap_or(Resources {
                mana: 0,
                thresholds: Thresholds::ZERO,
            });
        let health = data
            .avatar_health
            .get(&self.player_id)
            .copied()
            .unwrap_or(0);
        let deaths_door = health == 0;
        if deaths_door && self.deaths_door_texture.is_none() {
            let image = image::load_from_memory(DEATHS_DOOR_MEDALLION)
                .expect("Death's Door medallion should decode")
                .to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            self.deaths_door_texture = Some(ctx.load_texture(
                "deaths_door_reaper_v3",
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                egui::TextureOptions::LINEAR,
            ));
        }
        let deaths_door_texture = self.deaths_door_texture.clone();
        let hand_count = data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id && c.zone == Zone::Hand)
            .count();
        let banish_count = data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id && c.zone == Zone::Banish)
            .count();
        let grave_count = data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id && c.zone == Zone::Cemetery)
            .count();
        let is_self = data.player_id == self.player_id;
        let area_id = if self.player { "ps_self" } else { "ps_opp" };
        let mut open_hand = false;
        let mut open_cemetery = false;
        let mut open_banish = false;
        let can_view_controlled_hand =
            data.current_player == data.player_id && data.turn_player == self.player_id && !is_self;

        egui::Area::new(egui::Id::new(area_id))
            .fixed_pos(panel_pos)
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                draw_seat_housing(ui.painter(), panel_rect, is_self);
                egui::Frame::new()
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(egui::Margin::symmetric(PAD_H, PAD_V))
                    .show(ui, |ui| {
                        ui.set_min_width(panel_w - (PAD_H as f32) * 2.0);
                        ui.set_max_width(panel_w - (PAD_H as f32) * 2.0);
                        ui.vertical_centered(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 3.0);
                            seat_crest(ui, is_self);
                            ui.add_space(8.0);
                            life_vial(ui, health, deaths_door, deaths_door_texture.as_ref());
                            ui.add_space(10.0);
                            side_stat(
                                ui,
                                StatIcon::Mana,
                                "MANA",
                                resources.mana,
                                Color32::from_rgb(80, 200, 230),
                                if self.player { "mana_self" } else { "mana_opp" },
                                false,
                            );
                            if side_stat(
                                ui,
                                StatIcon::Hand,
                                "HAND",
                                hand_count,
                                Color32::from_rgb(230, 210, 100),
                                if self.player { "hand_self" } else { "hand_opp" },
                                can_view_controlled_hand,
                            )
                            .clicked()
                                && can_view_controlled_hand
                            {
                                open_hand = true;
                            }
                            if side_stat(
                                ui,
                                StatIcon::Cemetery,
                                "CEMETERY",
                                grave_count,
                                Color32::from_rgb(170, 170, 190),
                                if self.player {
                                    "grave_self"
                                } else {
                                    "grave_opp"
                                },
                                true,
                            )
                            .clicked()
                            {
                                open_cemetery = true;
                            }
                            if side_stat(
                                ui,
                                StatIcon::Banish,
                                "BANISH",
                                banish_count,
                                theme::ELEMENT_AIR,
                                if self.player {
                                    "banish_self"
                                } else {
                                    "banish_opp"
                                },
                                true,
                            )
                            .clicked()
                            {
                                open_banish = true;
                            }

                            ui.add_space(10.0);
                            threshold_grid(ui, resources.thresholds);
                        });
                    });
            });

        if open_cemetery {
            let title = if data.player_id == self.player_id {
                "Your Cemetery".to_string()
            } else {
                "Opponent's Cemetery".to_string()
            };
            return Ok(Some(ComponentCommand::OpenCardViewer {
                title,
                zone: Zone::Cemetery,
                controller_id: Some(self.player_id),
                mode: crate::components::CardViewerMode::Manual,
                open_only: false,
            }));
        }

        if open_hand {
            return Ok(Some(ComponentCommand::OpenCardViewer {
                title: "Controlled Player's Hand".to_string(),
                zone: Zone::Hand,
                controller_id: Some(self.player_id),
                mode: crate::components::CardViewerMode::Manual,
                open_only: true,
            }));
        }

        if open_banish {
            let title = if data.player_id == self.player_id {
                "Your Banished Cards".to_string()
            } else {
                "Opponent's Banished Cards".to_string()
            };
            return Ok(Some(ComponentCommand::OpenCardViewer {
                title,
                zone: Zone::Banish,
                controller_id: Some(self.player_id),
                mode: crate::components::CardViewerMode::Manual,
                open_only: false,
            }));
        }

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
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        if let ComponentCommand::SetRect {
            component_type: ComponentType::PlayerStatus,
            rect,
        } = command
        {
            self.rect = *rect;
        }
        Ok(())
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::PlayerStatus
    }
}
