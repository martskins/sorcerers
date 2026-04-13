use egui::epaint::CornerRadius;
use egui::{Color32, Context, Painter, Rect, Stroke, Ui, pos2, vec2};
use sorcerers::game::PlayerId;
use sorcerers::{
    card::Zone,
    game::{Element, Resources, Thresholds},
};

use crate::{
    components::{Component, ComponentCommand, ComponentType},
    element_icon,
    scene::game::GameData,
    texture_cache::TextureCache,
};

// ── Layout ─────────────────────────────────────────────────────────────────────
const ICON_SZ: f32 = 14.0;
const STAT_FONT: f32 = 14.0;
const NAME_FONT: f32 = 13.0;
const THRESH_SYM: f32 = 14.0; // triangle bounding box size
const PAD_H: i8 = 8; // horizontal inner margin (i8 for egui::Margin)
const PAD_V: i8 = 6; // vertical   inner margin

// Panel background / border
const BG: Color32 = Color32::from_rgba_premultiplied(15, 20, 38, 235);
const BORDER: Color32 = Color32::from_rgb(55, 70, 110);

#[derive(Debug)]
pub struct PlayerStatusComponent {
    visible: bool,
    player_id: PlayerId,
    /// `true` = local player (bottom of sidebar), `false` = opponent (top).
    player: bool,
    rect: Rect,
    pending_open_log: bool,
}

impl PlayerStatusComponent {
    pub fn new(rect: Rect, player_id: PlayerId, player: bool) -> Self {
        Self {
            visible: true,
            player_id,
            rect,
            player,
            pending_open_log: false,
        }
    }
}

/// Icon + number stat cell, laid out horizontally.
fn stat_cell(
    ui: &mut Ui,
    icon_path: &str,
    value: impl std::fmt::Display,
    color: Color32,
    ctx: &Context,
) {
    let icon_sz = vec2(ICON_SZ, ICON_SZ);
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
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(value.to_string())
            .color(color)
            .size(STAT_FONT)
            .strong(),
    );
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

impl Component for PlayerStatusComponent {
    fn update(&mut self, _data: &mut GameData, _ctx: &Context) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        _painter: &Painter,
    ) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }

        let sidebar_w = crate::config::realm_rect()
            .map(|r| r.min.x)
            .unwrap_or(220.0);
        let sr = crate::config::screen_rect()?;
        let ctx = ui.ctx().clone();

        // Panel sits at the very bottom (player) or very top (opponent) of the sidebar.
        let panel_y = if self.player {
            sr.height() - crate::config::SIDEBAR_PANEL_RESERVED - 4.0
        } else {
            4.0
        };
        let panel_w = sidebar_w - 18.0;

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
        let hand_count = data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id && c.zone == Zone::Hand)
            .count();
        let grave_count = data
            .cards
            .iter()
            .filter(|c| c.owner_id == self.player_id && c.zone == Zone::Cemetery)
            .count();
        let is_self = data.player_id == self.player_id;
        let name = if is_self { "YOU" } else { "OPPONENT" };
        let unseen = data.unseen_events;

        let area_id = if self.player { "ps_self" } else { "ps_opp" };
        let mut open_log = false;

        egui::Area::new(egui::Id::new(area_id))
            .fixed_pos(pos2(4.0, panel_y))
            .order(egui::Order::Background)
            .show(&ctx, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(egui::Margin::symmetric(PAD_H, PAD_V))
                    .show(ui, |ui| {
                        ui.set_min_width(panel_w - (PAD_H as f32) * 2.0);
                        ui.set_max_width(panel_w - (PAD_H as f32) * 2.0);

                        // ── Name + event-log link ─────────────────────────────
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(name)
                                    .color(Color32::from_rgb(190, 210, 255))
                                    .size(NAME_FONT)
                                    .strong(),
                            );
                            if is_self {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (label, col) = if unseen > 0 {
                                            // ✉ = envelope (U+2709, Dingbats)
                                            (
                                                format!("✉ {unseen}"),
                                                Color32::from_rgb(100, 200, 255),
                                            )
                                        } else {
                                            // ☰ = trigram / log icon (U+2630, Misc Symbols)
                                            ("☰ log".into(), Color32::from_rgb(100, 100, 130))
                                        };
                                        if ui
                                            .link(egui::RichText::new(label).color(col).size(12.0))
                                            .clicked()
                                        {
                                            open_log = true;
                                        }
                                    },
                                );
                            }
                        });

                        ui.add_space(2.0);
                        // Thin manual separator
                        let sep_rect = ui.available_rect_before_wrap();
                        let sep_y = sep_rect.min.y + 1.0;
                        ui.painter().line_segment(
                            [pos2(sep_rect.min.x, sep_y), pos2(sep_rect.max.x, sep_y)],
                            Stroke::new(1.0, BORDER),
                        );
                        ui.add_space(5.0);

                        // ── Stats: HP · Mana · Hand · Grave ──────────────────
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                            stat_cell(
                                ui,
                                "assets/icons/heart.png",
                                health,
                                Color32::from_rgb(230, 80, 80),
                                &ctx,
                            );
                            ui.add_space(5.0);
                            stat_cell(
                                ui,
                                "assets/icons/potion.png",
                                resources.mana,
                                Color32::from_rgb(80, 200, 230),
                                &ctx,
                            );
                            ui.add_space(5.0);
                            stat_cell(
                                ui,
                                "assets/icons/cards.png",
                                hand_count,
                                Color32::from_rgb(230, 210, 100),
                                &ctx,
                            );
                            ui.add_space(5.0);
                            stat_cell(
                                ui,
                                "assets/icons/tombstone.png",
                                grave_count,
                                Color32::from_rgb(170, 170, 190),
                                &ctx,
                            );
                        });

                        ui.add_space(4.0);

                        // ── Elemental thresholds ──────────────────────────────
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                            element_symbol(ui, resources.thresholds.fire, Element::Fire);
                            element_symbol(ui, resources.thresholds.air, Element::Air);
                            element_symbol(ui, resources.thresholds.earth, Element::Earth);
                            element_symbol(ui, resources.thresholds.water, Element::Water);
                        });
                    });
            });

        if open_log {
            self.pending_open_log = true;
        }
        Ok(())
    }

    fn process_input(
        &mut self,
        _in_turn: bool,
        _data: &mut GameData,
        _ctx: &Context,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if self.pending_open_log {
            self.pending_open_log = false;
            return Ok(Some(ComponentCommand::SetVisibility {
                component_type: ComponentType::EventLog,
                visible: true,
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
