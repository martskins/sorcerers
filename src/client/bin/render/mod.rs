use crate::{
    config::{CARD_ASPECT_RATIO, realm_rect, screen_rect},
    texture_cache::TextureCache,
};
use egui::{
    Color32, Context, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, Vec2,
    epaint::{Mesh, Shape, Vertex},
    pos2, vec2,
};
use sorcerers::card::{Ability, CardData, CardType};

#[derive(Clone)]
pub struct CardRect {
    pub rect: Rect,
    pub image: Option<TextureHandle>,
    pub is_hovered: bool,
    pub is_selected: bool,
    pub card: CardData,
}

impl std::fmt::Debug for CardRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardRect")
            .field("rect", &self.rect)
            .field("image", &self.image.as_ref().map(|_| "<TextureHandle>"))
            .field("is_hovered", &self.is_hovered)
            .field("is_selected", &self.is_selected)
            .field("card", &self.card)
            .finish()
    }
}

impl CardRect {
    pub fn rotation(&self) -> f32 {
        if self.card.tapped {
            return std::f32::consts::FRAC_PI_2;
        }
        0.0
    }
}

#[derive(Debug, Clone)]
pub struct CellRect {
    pub id: u8,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct IntersectionRect {
    pub locations: Vec<u8>,
    pub rect: Rect,
}

fn draw_vortex_icon(painter: &Painter, x: f32, y: f32, size: f32, color: Color32) {
    let turns = 2.0;
    let segments = 24;
    let mut prev = pos2(x + size / 2.0, y + size / 2.0);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = turns * std::f32::consts::TAU * t;
        let radius = (size / 2.0) * t;
        let px = x + size / 2.0 + radius * angle.cos();
        let py = y + size / 2.0 + radius * angle.sin();
        painter.line_segment([prev, pos2(px, py)], Stroke::new(2.0, color));
        prev = pos2(px, py);
    }
}

fn draw_rotated_image(
    painter: &Painter,
    tex_handle: &TextureHandle,
    rect: Rect,
    angle: f32,
    tint: Color32,
) {
    let cx = rect.center();
    let (sin, cos) = angle.sin_cos();
    let rotate =
        |v: Vec2| -> Pos2 { pos2(cos * v.x - sin * v.y + cx.x, sin * v.x + cos * v.y + cx.y) };
    let half = rect.size() * 0.5;
    let corners = [
        rotate(vec2(-half.x, -half.y)),
        rotate(vec2(half.x, -half.y)),
        rotate(vec2(half.x, half.y)),
        rotate(vec2(-half.x, half.y)),
    ];
    let uvs = [
        pos2(0.0, 0.0),
        pos2(1.0, 0.0),
        pos2(1.0, 1.0),
        pos2(0.0, 1.0),
    ];
    let mut mesh = Mesh::with_texture(tex_handle.id());
    for (c, uv) in corners.iter().zip(uvs.iter()) {
        mesh.vertices.push(Vertex {
            pos: *c,
            uv: *uv,
            color: tint,
        });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(Shape::mesh(mesh));
}

fn draw_card_internal(
    card_rect: &CardRect,
    is_ally: bool,
    draw_accessories: bool,
    painter: &Painter,
    rotation: f32,
) {
    let rect = card_rect.rect;
    let scale = if card_rect.is_hovered || card_rect.is_selected {
        1.1f32
    } else {
        1.0f32
    };
    let scaled_size = vec2(rect.width() * scale, rect.height() * scale);
    let scaled_rect = Rect::from_min_size(rect.min, scaled_size);

    if let Some(ref tex) = card_rect.image {
        let tint = if card_rect.card.abilities.contains(&Ability::Stealth) {
            Color32::from_rgba_unmultiplied(255, 255, 255, 217)
        } else {
            Color32::WHITE
        };
        if rotation == 0.0 {
            painter.image(
                tex.id(),
                scaled_rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                tint,
            );
        } else {
            // Rotate the image 90° around the card's centre.  We keep the
            // *original* portrait rect as the mesh base; draw_rotated_image
            // will rotate its four corners, producing a landscape footprint.
            draw_rotated_image(painter, tex, scaled_rect, rotation, tint);
        }
    } else {
        painter.rect_filled(scaled_rect, 4.0, Color32::DARK_GRAY);
    }

    let sleeve_color = if card_rect.is_selected {
        Color32::WHITE
    } else if is_ally {
        Color32::DARK_GREEN
    } else {
        Color32::RED
    };

    if card_rect.card.has_attachments {
        let triangle_w = 6.0;
        let triangle_h = 20.0;
        let card_w = rect.width() * scale;
        let card_h = rect.height() * scale;
        let card_center_x = rect.min.x + card_w / 2.0;
        let card_center_y = rect.min.y + card_h / 2.0;
        let (sin, cos) = rotation.sin_cos();
        let rotate = |vx: f32, vy: f32| -> Pos2 {
            pos2(
                cos * vx - sin * vy + card_center_x,
                sin * vx + cos * vy + card_center_y,
            )
        };
        let left_cx = -card_w / 2.0;
        let tip = rotate(left_cx - triangle_w, 0.0);
        let top = rotate(left_cx, -triangle_h / 2.0);
        let bot = rotate(left_cx, triangle_h / 2.0);
        painter.add(Shape::convex_polygon(
            vec![tip, top, bot],
            Color32::WHITE,
            Stroke::NONE,
        ));
        let right_cx = -card_w / 2.0 + 0.5;
        let rtip = rotate(right_cx + triangle_w, 0.0);
        let rtop = rotate(right_cx, -triangle_h / 2.0);
        let rbot = rotate(right_cx, triangle_h / 2.0);
        painter.add(Shape::convex_polygon(
            vec![rtip, rtop, rbot],
            Color32::WHITE,
            Stroke::NONE,
        ));
    }

    let w = rect.width() * scale;
    let h = rect.height() * scale;
    let cx = rect.min.x + w / 2.0;
    let cy = rect.min.y + h / 2.0;
    let corners_raw = [
        vec2(-w / 2.0, -h / 2.0),
        vec2(w / 2.0, -h / 2.0),
        vec2(w / 2.0, h / 2.0),
        vec2(-w / 2.0, h / 2.0),
    ];
    let (sin, cos) = rotation.sin_cos();
    let rotated: Vec<Pos2> = corners_raw
        .iter()
        .map(|v| pos2(cos * v.x - sin * v.y + cx, sin * v.x + cos * v.y + cy))
        .collect();
    painter.add(Shape::closed_line(rotated, Stroke::new(2.0, sleeve_color)));

    if card_rect
        .card
        .abilities
        .contains(&Ability::SummoningSickness)
    {
        let icon_size = 22.0;
        let x = card_rect.rect.min.x + card_rect.rect.width() * scale - icon_size - 4.0;
        let y = card_rect.rect.min.y + 4.0;
        draw_vortex_icon(painter, x, y, icon_size, Color32::BLUE);
    }

    if card_rect.card.abilities.contains(&Ability::Disabled) {
        let icon_size = 15.0;
        let x = card_rect.rect.min.x + card_rect.rect.width() - 30.0 - 5.0;
        let y = card_rect.rect.min.y + 4.0;
        let center = pos2(x + icon_size / 2.0, y + icon_size / 2.0);
        painter.circle_stroke(center, icon_size / 2.0, Stroke::new(3.0, Color32::WHITE));
        painter.line_segment(
            [
                pos2(x + 4.0, y + icon_size - 4.0),
                pos2(x + icon_size - 4.0, y + 4.0),
            ],
            Stroke::new(3.0, Color32::WHITE),
        );
    }

    if card_rect.card.zone.is_in_play() {
        if card_rect.card.card_type != CardType::Avatar
            && card_rect.card.damage_taken > 0
            && card_rect.card.zone.is_in_play()
        {
            let circle_radius = 8.0;
            let circle_pos = pos2(
                rect.min.x + circle_radius / 2.0,
                rect.min.y + circle_radius / 2.0,
            );
            painter.circle_filled(circle_pos, circle_radius - 2.0, Color32::RED);
            let dmg_text = card_rect.card.damage_taken.to_string();
            painter.text(
                circle_pos,
                egui::Align2::CENTER_CENTER,
                &dmg_text,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
        }

        if card_rect.card.card_type.is_unit() {
            let circle_radius = 8.0;
            let circle_pos = pos2(
                rect.min.x + w - circle_radius / 2.0,
                rect.min.y + circle_radius / 2.0,
            );
            painter.circle_filled(circle_pos, circle_radius - 2.0, Color32::BLUE);
            let power_text = card_rect.card.power.to_string();
            painter.text(
                circle_pos,
                egui::Align2::CENTER_CENTER,
                &power_text,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }

    if draw_accessories && card_rect.is_selected {
        painter.rect_stroke(
            scaled_rect,
            0.0,
            Stroke::new(2.0, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
}

pub fn draw_card(card_rect: &CardRect, is_ally: bool, draw_accessories: bool, painter: &Painter) {
    draw_card_internal(
        card_rect,
        is_ally,
        draw_accessories,
        painter,
        card_rect.rotation(),
    );
}

pub fn draw_card_with_rotation(
    card_rect: &CardRect,
    is_ally: bool,
    draw_accessories: bool,
    painter: &Painter,
    rotation: f32,
) {
    draw_card_internal(card_rect, is_ally, draw_accessories, painter, rotation);
}

/// Draws a bigger version of a card on the given position.
pub fn draw_card_preview(
    tex: Option<&TextureHandle>,
    pos: Pos2,
    painter: &Painter,
) -> anyhow::Result<()> {
    let mut preview_size = vec2(200.0, 200.0 / CARD_ASPECT_RATIO);
    if let Some(tex) = tex {
        // If the texture is wider than it is tall, it's a site.
        if tex.aspect_ratio() > 1.0 {
            std::mem::swap(&mut preview_size.x, &mut preview_size.y);
        }
    }
    let rect = Rect::from_min_size(pos, preview_size);
    draw_card_preview_internal(tex, rect, painter)
}

/// Draws a bigger version of a card on the left sidebar, vertically centred on screen.
pub fn draw_sidebar_card_preview(
    tex: Option<&TextureHandle>,
    painter: &Painter,
) -> anyhow::Result<()> {
    const MARGIN: f32 = 4.0;
    let available_w = realm_rect()?.min.x - MARGIN * 2.0;
    let mut preview_size = vec2(available_w, available_w / CARD_ASPECT_RATIO);
    if let Some(tex) = tex {
        // If the texture is wider than it is tall, it's a site.
        if tex.aspect_ratio() > 1.0 {
            std::mem::swap(&mut preview_size.x, &mut preview_size.y);
        }
    }
    let preview_y = screen_rect()?.height() / 2.0 - preview_size.y / 2.0;
    let dest_rect = Rect::from_min_size(pos2(MARGIN, preview_y), preview_size);
    draw_card_preview_internal(tex, dest_rect, painter)
}

fn draw_card_preview_internal(
    tex: Option<&TextureHandle>,
    dest_rect: Rect,
    painter: &Painter,
) -> anyhow::Result<()> {
    if let Some(tex) = tex {
        painter.image(
            tex.id(),
            dest_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(dest_rect, 4.0, Color32::DARK_GRAY);
    }

    Ok(())
}

pub fn wrap_text<S: AsRef<str>>(text: S, _max_width: f32, _font_size: u16) -> String {
    text.as_ref().to_string()
}

pub fn popup_action_menu(
    ctx: &egui::Context,
    anchor: Option<Pos2>,
    prompt: &str,
    options: &[String],
    painter: &Painter,
) -> Option<usize> {
    let mut result: Option<usize> = None;

    // ── Layout constants ──────────────────────────────────────────
    const MENU_W: f32 = 230.0;
    const HEADER_H: f32 = 36.0;
    const ROW_H: f32 = 46.0;
    const CORNER: f32 = 10.0;
    const ACCENT: Color32 = Color32::from_rgb(90, 160, 255);
    const BG: Color32 = Color32::from_rgb(14, 16, 28);
    const BG_ROW_HOVER: Color32 = Color32::from_rgb(35, 55, 100);
    const SEP: Color32 = Color32::from_rgb(40, 44, 68);
    const PADDING_X: f32 = 20.0;

    let total_h = HEADER_H + options.len() as f32 * ROW_H + 2.0;
    let screen = screen_rect().unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0)));
    let origin = if let Some(card_pos) = anchor {
        let mut x = card_pos.x + 60.0;
        let mut y = card_pos.y - total_h / 2.0;
        if x + MENU_W > screen.max.x - 8.0 {
            x = card_pos.x - MENU_W - 60.0;
        }
        y = y.clamp(screen.min.y + 8.0, screen.max.y - total_h - 8.0);
        pos2(x, y)
    } else {
        pos2(
            (screen.width() - MENU_W) / 2.0,
            (screen.height() - total_h) / 2.0,
        )
    };

    // ── Determine position ────────────────────────────────────────
    let title = painter.fonts(|f| {
        f.layout_no_wrap(
            prompt.to_string(),
            egui::FontId::proportional(14.0),
            Color32::from_rgb(180, 200, 240),
        )
    });

    let option_galleys = options
        .iter()
        .map(|option| {
            painter.fonts(|f| {
                f.layout_no_wrap(
                    option.clone(),
                    egui::FontId::proportional(18.0),
                    Color32::from_rgb(200, 210, 230),
                )
            })
        })
        .collect::<Vec<_>>();

    let mut total_w = title.size().x;
    for og in &option_galleys {
        if og.size().x > total_w {
            total_w = og.size().x;
        }
    }
    total_w += PADDING_X;

    // ── Draw via Area so we control every pixel ───────────────────
    egui::Area::new(egui::Id::new("action_menu_popup"))
        .fixed_pos(origin)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let menu_rect = Rect::from_min_size(origin, vec2(total_w, total_h));
            let p = ui.painter();

            // Drop shadow
            p.rect_filled(
                menu_rect.translate(vec2(4.0, 4.0)),
                egui::CornerRadius::same(CORNER as u8),
                Color32::from_black_alpha(120),
            );
            // Main background
            p.rect_filled(menu_rect, egui::CornerRadius::same(CORNER as u8), BG);
            // Accent border
            p.rect_stroke(
                menu_rect,
                egui::CornerRadius::same(CORNER as u8),
                egui::Stroke::new(1.5, ACCENT),
                egui::StrokeKind::Outside,
            );
            // Header accent bar
            let header_rect = Rect::from_min_size(origin, vec2(total_w, HEADER_H));
            p.rect_filled(
                header_rect,
                egui::CornerRadius {
                    nw: CORNER as u8,
                    ne: CORNER as u8,
                    sw: 0,
                    se: 0,
                },
                Color32::from_rgb(22, 32, 60),
            );
            // Header separator line
            p.hline(
                origin.x..=origin.x + total_w,
                origin.y + HEADER_H,
                egui::Stroke::new(1.0, ACCENT),
            );
            p.galley(
                pos2(origin.x + PADDING_X / 2.0, origin.y + title.size().y / 2.0),
                title,
                Color32::WHITE,
            );

            // Action rows
            for (idx, _option) in options.iter().enumerate() {
                let row_y = origin.y + HEADER_H + idx as f32 * ROW_H;
                let row_rect =
                    Rect::from_min_size(pos2(origin.x + 1.0, row_y), vec2(total_w - 2.0, ROW_H));
                // Last row gets rounded bottom corners
                let row_cr = if idx + 1 == options.len() {
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: CORNER as u8,
                        se: CORNER as u8,
                    }
                } else {
                    egui::CornerRadius::ZERO
                };

                let resp = ui.interact(
                    row_rect,
                    egui::Id::new(("action_row", idx)),
                    egui::Sense::click(),
                );
                if resp.hovered() {
                    p.rect_filled(row_rect, row_cr, BG_ROW_HOVER);
                }
                if resp.clicked() {
                    result = Some(idx);
                }

                // Separator (skip before first row)
                if idx > 0 {
                    p.hline(
                        origin.x + 12.0..=origin.x + total_w - 12.0,
                        row_y,
                        egui::Stroke::new(0.5, SEP),
                    );
                }

                // Arrow glyph
                p.text(
                    pos2(origin.x + 18.0, row_y + ROW_H / 2.0),
                    egui::Align2::LEFT_CENTER,
                    "▸",
                    egui::FontId::proportional(14.0),
                    if resp.hovered() {
                        ACCENT
                    } else {
                        Color32::from_rgb(80, 100, 140)
                    },
                );

                // Action label
                let galley = option_galleys[idx].clone();
                p.galley_with_override_text_color(
                    pos2(origin.x + 36.0, row_y + galley.size().y / 2.0),
                    galley,
                    if resp.hovered() {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(200, 210, 230)
                    },
                );
            }
        });

    result
}
