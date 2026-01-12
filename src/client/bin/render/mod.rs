use crate::{
    config::realm_rect,
    scene::game::{GameData, Status},
};
use macroquad::{
    color::{BLUE, Color, DARKGREEN, RED, WHITE},
    math::{Rect, Vec2},
    shapes::{draw_circle, draw_circle_lines, draw_line, draw_rectangle_lines},
    text::draw_text,
    texture::{DrawTextureParams, Texture2D, draw_texture_ex},
    ui,
};
use sorcerers::card::{Ability, CardData, CardType};

#[derive(Debug, Clone)]
pub struct CardRect {
    pub rect: Rect,
    pub image: Texture2D,
    pub is_hovered: bool,
    pub is_selected: bool,
    pub card: CardData,
}

impl CardRect {
    pub fn rotation(&self) -> f32 {
        if self.card.tapped {
            return std::f32::consts::FRAC_PI_2;
        }

        return 0.0;
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

fn draw_vortex_icon(x: f32, y: f32, size: f32, color: Color) {
    let turns = 2.0;
    let segments = 24;
    let mut prev = (x + size / 2.0, y + size / 2.0);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = turns * std::f32::consts::TAU * t;
        let radius = (size / 2.0) * t;
        let px = x + size / 2.0 + radius * angle.cos();
        let py = y + size / 2.0 + radius * angle.sin();
        macroquad::shapes::draw_line(prev.0, prev.1, px, py, 2.0, color);
        prev = (px, py);
    }
}

pub fn draw_card(card_rect: &CardRect, is_ally: bool, draw_accessories: bool) {
    let rect = card_rect.rect;
    let mut scale = 1.0;
    if card_rect.is_hovered || card_rect.is_selected {
        scale = 1.1;
    }

    let mut color = WHITE;
    if card_rect.card.abilities.contains(&Ability::Stealth) {
        color = Color::new(1.0, 1.0, 1.0, 0.85);
    }

    draw_texture_ex(
        &card_rect.image,
        rect.x,
        rect.y,
        color,
        DrawTextureParams {
            dest_size: Some(Vec2::new(rect.w, rect.h) * scale),
            rotation: card_rect.rotation(),
            ..Default::default()
        },
    );

    let mut sleeve_color = DARKGREEN;
    if !is_ally {
        sleeve_color = RED;
    }

    if card_rect.card.has_attachments {
        let triangle_w = 6.0;
        let triangle_h = 20.0;
        let card_w = rect.w * scale;
        let card_h = rect.h * scale;
        let card_center_x = rect.x + card_w / 2.0;
        let card_center_y = rect.y + card_h / 2.0;
        let (sin, cos) = card_rect.rotation().sin_cos();

        let left_center = Vec2::new(-card_w / 2.0, 0.0);
        let left_tip = left_center + Vec2::new(-triangle_w, 0.0);
        let left_top = left_center + Vec2::new(0.0, -triangle_h / 2.0);
        let left_bot = left_center + Vec2::new(0.0, triangle_h / 2.0);

        let rotate = |v: Vec2| {
            Vec2::new(
                cos * v.x - sin * v.y + card_center_x,
                sin * v.x + cos * v.y + card_center_y,
            )
        };

        let tip = rotate(left_tip);
        let top = rotate(left_top);
        let bot = rotate(left_bot);

        macroquad::shapes::draw_triangle(tip, top, bot, WHITE);

        // Right triangle (points right)
        let right_center = Vec2::new(-card_w / 2.0 + 0.5, 0.0);
        let right_tip = right_center + Vec2::new(triangle_w, 0.0);
        let right_top = right_center + Vec2::new(0.0, -triangle_h / 2.0);
        let right_bot = right_center + Vec2::new(0.0, triangle_h / 2.0);

        let rtip = rotate(right_tip);
        let rtop = rotate(right_top);
        let rbot = rotate(right_bot);

        macroquad::shapes::draw_triangle(rtip, rtop, rbot, WHITE);
    }

    // Draw rectangle border rotated around the center
    let w = rect.w * scale;
    let h = rect.h * scale;
    let cx = rect.x + w / 2.0;
    let cy = rect.y + h / 2.0;
    let corners = [
        Vec2::new(-w / 2.0, -h / 2.0),
        Vec2::new(w / 2.0, -h / 2.0),
        Vec2::new(w / 2.0, h / 2.0),
        Vec2::new(-w / 2.0, h / 2.0),
    ];
    let rotated: Vec<Vec2> = corners
        .iter()
        .map(|corner| {
            let (sin, cos) = card_rect.rotation().sin_cos();
            Vec2::new(
                cos * corner.x - sin * corner.y + cx,
                sin * corner.x + cos * corner.y + cy,
            )
        })
        .collect();
    for i in 0..4 {
        draw_line(
            rotated[i].x,
            rotated[i].y,
            rotated[(i + 1) % 4].x,
            rotated[(i + 1) % 4].y,
            2.0,
            sleeve_color,
        );
    }

    if card_rect.card.abilities.contains(&Ability::SummoningSickness) {
        let icon_size = 22.0;
        let scale = 1.0;
        let x = card_rect.rect.x + card_rect.rect.w * scale - icon_size - 4.0;
        let y = card_rect.rect.y + 4.0;
        draw_vortex_icon(x, y, icon_size, BLUE);
    }

    if card_rect.card.abilities.contains(&Ability::Disabled) {
        let icon_size = 15.0;
        let x = card_rect.rect.x + card_rect.rect.w - 30.0 - 5.0;
        let y = card_rect.rect.y + 4.0;
        let cx = x + icon_size / 2.0;
        let cy = y + icon_size / 2.0;
        draw_circle_lines(cx, cy, icon_size / 2.0, 3.0, WHITE);
        draw_line(x + 4.0, y + icon_size - 4.0, x + icon_size - 4.0, y + 4.0, 3.0, WHITE);
    }

    // Draw red circle with damage taken in top-left corner if the card has taken damage and is a
    // minion.
    if card_rect.card.card_type != CardType::Avatar
        && card_rect.card.damage_taken > 0
        && card_rect.card.zone.is_in_play()
    {
        let circle_radius = 8.0;
        let circle_x = rect.x + (circle_radius / 2.0);
        let circle_y = rect.y + (circle_radius / 2.0);
        draw_circle(circle_x, circle_y, circle_radius - 2.0, RED);
        let dmg_text = card_rect.card.damage_taken.to_string();
        let text_dims = macroquad::text::measure_text(&dmg_text, None, 12, 1.0);
        draw_text(
            &dmg_text,
            circle_x - text_dims.width / 2.0,
            circle_y + text_dims.height / 2.8,
            12.0,
            WHITE,
        );
    }

    // Draw power on a blue circle in the top-right corner if the card is a unit.
    if card_rect.card.card_type.is_unit() {
        let circle_radius = 8.0;
        let circle_x = rect.x + w - (circle_radius / 2.0);
        let circle_y = rect.y + (circle_radius / 2.0);
        draw_circle(circle_x, circle_y, circle_radius - 2.0, BLUE);
        let dmg_text = card_rect.card.power.to_string();
        let text_dims = macroquad::text::measure_text(&dmg_text, None, 12, 1.0);
        draw_text(
            &dmg_text,
            circle_x - text_dims.width / 2.0,
            circle_y + text_dims.height / 2.8,
            12.0,
            WHITE,
        );
    }

    if draw_accessories && card_rect.is_selected {
        draw_rectangle_lines(rect.x, rect.y, rect.w * scale, rect.h * scale, 2.0, WHITE);
    }
}

pub async fn render_card_preview(card: &CardRect, data: &mut GameData) -> anyhow::Result<()> {
    if let Status::SelectingCard { preview: true, .. } = &data.status {
        return Ok(());
    }

    let screen_rect = crate::config::screen_rect()?;
    let mut rect = card.rect;
    let mut preview_scale: f32 = realm_rect()?.x / card.rect.w;
    if rect.w > rect.h {
        preview_scale = realm_rect()?.x / card.rect.h;
    }

    rect.w *= preview_scale;
    rect.h *= preview_scale;
    let preview_y = screen_rect.h / 2.0 - rect.h / 2.0;
    draw_texture_ex(
        &card.image,
        0.0,
        preview_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(rect.w, rect.h)),
            ..Default::default()
        },
    );

    Ok(())
}

pub fn menu_skin() -> ui::Skin {
    let button_style = ui::root_ui()
        .style_builder()
        .font_size(32)
        .text_color(WHITE)
        .text_color_hovered(WHITE)
        .text_color_clicked(WHITE)
        .color(macroquad::color::Color::from_rgba(30, 144, 255, 255))
        .color_hovered(macroquad::color::Color::from_rgba(65, 105, 225, 255))
        .color_clicked(macroquad::color::Color::from_rgba(25, 25, 112, 255))
        .build();
    let editbox_style = ui::root_ui().style_builder().font_size(30).build();
    let label_style = ui::root_ui().style_builder().font_size(30).text_color(WHITE).build();

    ui::Skin {
        button_style,
        editbox_style,
        label_style,
        ..ui::root_ui().default_skin()
    }
}

pub fn wrap_text<S: AsRef<str>>(text: S, max_width: f32, font_size: u16) -> String {
    use macroquad::text::measure_text;

    let mut lines = Vec::new();
    for paragraph in text.as_ref().split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let test = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            let dims = measure_text(&test, None, font_size, 1.0);
            if dims.width > max_width && !current.is_empty() {
                lines.push(current.clone());
                current = word.to_string();
            } else {
                current = test;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }

    lines.join("\n")
}

pub fn multiline_label(text: &str, pos: Vec2, font_size: u16, ui: &mut ui::Ui) {
    for (idx, line) in text.lines().enumerate() {
        let line_pos = pos + Vec2::new(0.0, idx as f32 * (font_size as f32 + 4.0));
        ui::widgets::Label::new(line).position(line_pos).ui(ui);
    }
}
