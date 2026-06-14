use egui::epaint::CornerRadius;
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
    texture_cache::TextureCache,
    theme,
};

// ── Layout ─────────────────────────────────────────────────────────────────────
const SIDE_ICON_SZ: f32 = 13.0;
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

fn status_icon(ui: &mut Ui, icon_path: &str, ctx: &Context, size: f32) {
    let icon_sz = vec2(size, size);
    if let Some(tex) = TextureCache::get_texture_blocking(icon_path, ctx) {
        let img = egui::Image::new(egui::ImageSource::Texture(
            egui::load::SizedTexture::from_handle(&tex),
        ))
        .max_size(icon_sz);
        ui.add(img);
    } else {
        let (r, _) = ui.allocate_exact_size(icon_sz, egui::Sense::hover());
        ui.painter()
            .rect_filled(r, 3.0, Color32::from_rgb(50, 50, 60));
    }
}

fn side_stat(
    ui: &mut Ui,
    icon_path: &str,
    value: impl std::fmt::Display,
    color: Color32,
    ctx: &Context,
    id: &'static str,
    clickable: bool,
) -> Response {
    let response = ui
        .push_id(id, |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(58.0, 23.0), egui::Sense::click());
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

            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(4.0);
                    status_icon(ui, icon_path, ctx, SIDE_ICON_SZ);
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(value.to_string())
                            .color(color)
                            .size(12.0)
                            .strong(),
                    );
                });
            });
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
    let (rect, _) = ui.allocate_exact_size(vec2(46.0, 62.0), egui::Sense::hover());
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

    let fill_h = rect.height() * fill_fraction;
    let fill_rect = Rect::from_min_max(
        pos2(rect.min.x + 4.0, rect.max.y - 4.0 - fill_h),
        pos2(rect.max.x - 4.0, rect.max.y - 4.0),
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
        FontId::proportional(18.0),
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
        let panel_w = 74.0;
        let top_margin = 72.0;
        let bottom_margin = 96.0;
        let panel_h = 242.0;
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
        let name = if is_self { "YOU" } else { "OPP" };
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
                                "assets/icons/potion.png",
                                resources.mana,
                                Color32::from_rgb(80, 200, 230),
                                &ctx,
                                if self.player { "mana_self" } else { "mana_opp" },
                                false,
                            );
                            if side_stat(
                                ui,
                                "assets/icons/cards.png",
                                hand_count,
                                Color32::from_rgb(230, 210, 100),
                                &ctx,
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
                                "assets/icons/tombstone.png",
                                grave_count,
                                Color32::from_rgb(170, 170, 190),
                                &ctx,
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
                                "assets/icons/banish.png",
                                banish_count,
                                Color32::from_rgb(170, 170, 190),
                                &ctx,
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
                card_ids: None,
                open_only: false,
            }));
        }

        if open_hand {
            return Ok(Some(ComponentCommand::OpenCardViewer {
                title: "Controlled Player's Hand".to_string(),
                zone: Zone::Hand,
                controller_id: Some(self.player_id),
                card_ids: None,
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
                card_ids: None,
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
