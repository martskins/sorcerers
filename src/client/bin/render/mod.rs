use crate::{
    config::realm_rect,
    scene::game::{GameData, Status},
};
use egui::{
    Color32, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, Vec2, pos2, vec2,
    epaint::{Mesh, Shape, Vertex},
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

fn draw_rotated_image(painter: &Painter, tex_handle: &TextureHandle, rect: Rect, angle: f32, tint: Color32) {
    let cx = rect.center();
    let (sin, cos) = angle.sin_cos();
    let rotate = |v: Vec2| -> Pos2 {
        pos2(cos * v.x - sin * v.y + cx.x, sin * v.x + cos * v.y + cx.y)
    };
    let half = rect.size() * 0.5;
    let corners = [
        rotate(vec2(-half.x, -half.y)),
        rotate(vec2(half.x, -half.y)),
        rotate(vec2(half.x, half.y)),
        rotate(vec2(-half.x, half.y)),
    ];
    let uvs = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)];
    let mut mesh = Mesh::with_texture(tex_handle.id());
    for (c, uv) in corners.iter().zip(uvs.iter()) {
        mesh.vertices.push(Vertex { pos: *c, uv: *uv, color: tint });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(Shape::mesh(mesh));
}

pub fn draw_card(card_rect: &CardRect, is_ally: bool, draw_accessories: bool, painter: &Painter) {
    let rect = card_rect.rect;
    let scale = if card_rect.is_hovered || card_rect.is_selected { 1.1f32 } else { 1.0f32 };
    let scaled_size = vec2(rect.width() * scale, rect.height() * scale);
    let scaled_rect = Rect::from_min_size(rect.min, scaled_size);

    if let Some(ref tex) = card_rect.image {
        let tint = if card_rect.card.abilities.contains(&Ability::Stealth) {
            Color32::from_rgba_unmultiplied(255, 255, 255, 217)
        } else {
            Color32::WHITE
        };
        if card_rect.rotation() == 0.0 {
            painter.image(tex.id(), scaled_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
        } else {
            // Rotate the image 90° around the card's centre.  We keep the
            // *original* portrait rect as the mesh base; draw_rotated_image
            // will rotate its four corners, producing a landscape footprint
            // (matching macroquad's draw_texture_ex rotation behaviour).
            draw_rotated_image(painter, tex, scaled_rect, std::f32::consts::FRAC_PI_2, tint);
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
        let (sin, cos) = card_rect.rotation().sin_cos();
        let rotate = |vx: f32, vy: f32| -> Pos2 {
            pos2(cos * vx - sin * vy + card_center_x, sin * vx + cos * vy + card_center_y)
        };
        let left_cx = -card_w / 2.0;
        let tip = rotate(left_cx - triangle_w, 0.0);
        let top = rotate(left_cx, -triangle_h / 2.0);
        let bot = rotate(left_cx, triangle_h / 2.0);
        painter.add(Shape::convex_polygon(vec![tip, top, bot], Color32::WHITE, Stroke::NONE));
        let right_cx = -card_w / 2.0 + 0.5;
        let rtip = rotate(right_cx + triangle_w, 0.0);
        let rtop = rotate(right_cx, -triangle_h / 2.0);
        let rbot = rotate(right_cx, triangle_h / 2.0);
        painter.add(Shape::convex_polygon(vec![rtip, rtop, rbot], Color32::WHITE, Stroke::NONE));
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
    let (sin, cos) = card_rect.rotation().sin_cos();
    let rotated: Vec<Pos2> = corners_raw.iter().map(|v| pos2(cos * v.x - sin * v.y + cx, sin * v.x + cos * v.y + cy)).collect();
    for i in 0..4 {
        painter.line_segment([rotated[i], rotated[(i + 1) % 4]], Stroke::new(2.0, sleeve_color));
    }

    if card_rect.card.abilities.contains(&Ability::SummoningSickness) {
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
            [pos2(x + 4.0, y + icon_size - 4.0), pos2(x + icon_size - 4.0, y + 4.0)],
            Stroke::new(3.0, Color32::WHITE),
        );
    }

    if card_rect.card.card_type != CardType::Avatar
        && card_rect.card.damage_taken > 0
        && card_rect.card.zone.is_in_play()
    {
        let circle_radius = 8.0;
        let circle_pos = pos2(rect.min.x + circle_radius / 2.0, rect.min.y + circle_radius / 2.0);
        painter.circle_filled(circle_pos, circle_radius - 2.0, Color32::RED);
        let dmg_text = card_rect.card.damage_taken.to_string();
        painter.text(circle_pos, egui::Align2::CENTER_CENTER, &dmg_text, FontId::proportional(10.0), Color32::WHITE);
    }

    if card_rect.card.card_type.is_unit() {
        let circle_radius = 8.0;
        let circle_pos = pos2(rect.min.x + w - circle_radius / 2.0, rect.min.y + circle_radius / 2.0);
        painter.circle_filled(circle_pos, circle_radius - 2.0, Color32::BLUE);
        let power_text = card_rect.card.power.to_string();
        painter.text(circle_pos, egui::Align2::CENTER_CENTER, &power_text, FontId::proportional(10.0), Color32::WHITE);
    }

    if draw_accessories && card_rect.is_selected {
        painter.rect_stroke(scaled_rect, 0.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
    }
}

pub fn render_card_preview(card: &CardRect, data: &mut GameData, painter: &Painter) -> anyhow::Result<()> {
    if let Status::SelectingCard { preview: true, .. } = &data.status {
        return Ok(());
    }

    let screen_rect = crate::config::screen_rect()?;
    let mut rect = card.rect;
    let mut preview_scale: f32 = realm_rect()?.min.x / card.rect.width();
    if rect.width() > rect.height() {
        preview_scale = realm_rect()?.min.x / card.rect.height();
    }

    rect = Rect::from_min_size(rect.min, rect.size() * preview_scale);
    let preview_y = screen_rect.height() / 2.0 - rect.height() / 2.0;
    let dest_rect = Rect::from_min_size(pos2(0.0, preview_y), rect.size());

    if let Some(ref tex) = card.image {
        painter.image(tex.id(), dest_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    } else {
        painter.rect_filled(dest_rect, 4.0, Color32::DARK_GRAY);
    }

    Ok(())
}

pub fn wrap_text<S: AsRef<str>>(text: S, _max_width: f32, _font_size: u16) -> String {
    text.as_ref().to_string()
}
