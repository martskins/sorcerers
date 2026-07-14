use egui::epaint::{CornerRadius, Shape};
use egui::{Align2, Color32, Context, FontId, Painter, Rect, Response, Stroke, Ui, pos2, vec2};
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
const NAME_FONT: f32 = 13.0;
const DEATHS_DOOR_FONT: f32 = 11.0;
const THRESH_SYM: f32 = 13.0; // triangle bounding box size
const PAD_H: i8 = 8; // horizontal inner margin (i8 for egui::Margin)
const PAD_V: i8 = 6; // vertical   inner margin

// Panel background / border
const BG: Color32 = theme::PANEL_BG;
const BORDER: Color32 = theme::PANEL_BORDER;

#[derive(Debug)]
pub struct PlayerStatusComponent {
    visible: bool,
    player_id: PlayerId,
    /// `true` = local player (left rail), `false` = opponent (right rail).
    player: bool,
    rect: Rect,
}

impl PlayerStatusComponent {
    pub fn new(rect: Rect, player_id: PlayerId, player: bool) -> Self {
        Self {
            visible: true,
            player_id,
            rect,
            player,
        }
    }
}

/// Official Sorcery: Contested Realm element symbol.
///
/// Fire  = upward triangle               (▲)
/// Air   = upward triangle + horizontal midline
/// Earth = downward triangle + horizontal midline
/// Water = downward triangle              (▽)
fn element_symbol(ui: &mut Ui, count: u8, element: Element) {
    element_icon::element_symbol_widget(ui, count, &element, THRESH_SYM, 1.5);
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
                [pos2(center.x, stone.min.y + 3.0), pos2(center.x, stone.max.y - 3.0)],
                stroke,
            );
            painter.line_segment(
                [pos2(center.x - 3.0, center.y + 2.0), pos2(center.x + 3.0, center.y + 2.0)],
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
            let (rect, response) = ui.allocate_exact_size(vec2(78.0, 25.0), egui::Sense::click());
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
            let icon_rect = Rect::from_center_size(
                pos2(rect.min.x + 14.0, rect.center().y),
                vec2(16.0, 16.0),
            );
            status_icon(ui.painter(), icon_rect, icon, color);
            ui.painter().text(
                pos2(rect.min.x + 28.0, rect.min.y + 4.0),
                Align2::LEFT_TOP,
                label,
                FontId::proportional(8.0),
                theme::TURN_WAITING,
            );
            ui.painter().text(
                pos2(rect.max.x - 8.0, rect.center().y + 3.0),
                Align2::RIGHT_CENTER,
                value.to_string(),
                FontId::proportional(14.0),
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

fn life_vial(ui: &mut Ui, health: u16, deaths_door: bool) {
    let max_health = 20.0;
    let fill_fraction = if deaths_door {
        0.05
    } else {
        (health as f32 / max_health).clamp(0.0, 1.0)
    };
    let (rect, _) = ui.allocate_exact_size(vec2(78.0, 54.0), egui::Sense::hover());
    let painter = ui.painter();
    let rounding = CornerRadius::same(8);
    painter.rect_filled(
        rect,
        rounding,
        Color32::from_rgba_unmultiplied(28, 18, 22, 230),
    );
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(
            1.0,
            if deaths_door {
                Color32::from_rgb(255, 210, 80)
            } else {
                Color32::from_rgb(130, 52, 58)
            },
        ),
        egui::StrokeKind::Outside,
    );

    painter.text(
        pos2(rect.center().x, rect.min.y + 8.0),
        Align2::CENTER_CENTER,
        "VITALITY",
        FontId::proportional(8.0),
        theme::TURN_WAITING,
    );
    let fill_w = (rect.width() - 10.0) * fill_fraction;
    let fill_rect = Rect::from_min_max(
        pos2(rect.min.x + 5.0, rect.max.y - 10.0),
        pos2(rect.min.x + 5.0 + fill_w, rect.max.y - 5.0),
    );
    let fill = if deaths_door {
        Color32::from_rgb(235, 184, 58)
    } else {
        Color32::from_rgb(176, 32, 48)
    };
    painter.rect_filled(fill_rect, CornerRadius::same(5), fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        health.to_string(),
        FontId::proportional(24.0),
        Color32::WHITE,
    );
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
        let panel_h = 264.0;
        let panel_y = if self.player {
            sr.max.y - bottom_margin - panel_h
        } else {
            top_margin
        };
        let panel_x = 12.0;
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
        let name = if is_self { "YOUR TABLE" } else { "OPPONENT" };
        let seat_label = if is_self { "YOUR SIDE" } else { "OPPOSING SIDE" };
        let unseen = data.unseen_events;

        let area_id = if self.player { "ps_self" } else { "ps_opp" };
        let mut open_log = false;
        let mut open_hand = false;
        let mut open_cemetery = false;
        let mut open_banish = false;
        let can_view_controlled_hand =
            data.current_player == data.player_id && data.turn_player == self.player_id && !is_self;

        egui::Area::new(egui::Id::new(area_id))
            .fixed_pos(panel_pos)
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(PAD_H, PAD_V))
                    .show(ui, |ui| {
                        ui.set_min_width(panel_w - (PAD_H as f32) * 2.0);
                        ui.set_max_width(panel_w - (PAD_H as f32) * 2.0);
                        ui.vertical_centered(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 3.0);
                            ui.label(
                                egui::RichText::new(name)
                                    .color(Color32::from_rgb(190, 210, 255))
                                    .size(NAME_FONT)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(seat_label)
                                    .color(theme::TURN_WAITING)
                                    .size(9.0),
                            );
                            ui.add_space(4.0);
                            life_vial(ui, health, deaths_door);
                            if deaths_door {
                                ui.label(
                                    egui::RichText::new("DOOR")
                                        .color(Color32::from_rgb(255, 210, 80))
                                        .size(DEATHS_DOOR_FONT)
                                        .strong(),
                                );
                            }
                            ui.add_space(4.0);
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

                            ui.add_space(3.0);
                            ui.vertical_centered(|ui| {
                                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                                ui.horizontal(|ui| {
                                    element_symbol(ui, resources.thresholds.fire, Element::Fire);
                                    element_symbol(ui, resources.thresholds.air, Element::Air);
                                });
                                ui.horizontal(|ui| {
                                    element_symbol(ui, resources.thresholds.earth, Element::Earth);
                                    element_symbol(ui, resources.thresholds.water, Element::Water);
                                });
                            });

                            if is_self {
                                ui.add_space(3.0);
                                let label = if unseen > 0 {
                                    format!("log {unseen}")
                                } else {
                                    "log".to_string()
                                };
                                if ui
                                    .link(
                                        egui::RichText::new(label)
                                            .color(Color32::from_rgb(100, 200, 255))
                                            .size(11.0),
                                    )
                                    .clicked()
                                {
                                    open_log = true;
                                }
                            }
                        });
                    });
            });

        if open_log {
            return Ok(Some(ComponentCommand::SetVisibility {
                component_type: ComponentType::EventLog,
                visible: true,
            }));
        }

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
