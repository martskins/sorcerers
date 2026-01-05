use macroquad::{
    color::{BLUE, Color, DARKGREEN, RED, WHITE},
    math::{Rect, Vec2},
    shapes::{draw_circle, draw_circle_lines, draw_line},
    text::draw_text,
    texture::{DrawTextureParams, Texture2D, draw_texture_ex},
    ui,
};
use sorcerers::{
    card::{CardType, Modifier, Zone},
    game::PlayerId,
};

use crate::{
    config::realm_rect,
    scene::game::{GameData, Status},
};

#[derive(Debug, Clone)]
pub struct CardRect {
    pub id: uuid::Uuid,
    pub owner_id: PlayerId,
    pub zone: Zone,
    pub tapped: bool,
    pub image: Texture2D,
    pub rect: Rect,
    pub is_hovered: bool,
    pub modifiers: Vec<Modifier>,
    pub damage_taken: u8,
    pub card_type: CardType,
}

impl CardRect {
    pub fn rotation(&self) -> f32 {
        if self.tapped {
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
    use macroquad::shapes::draw_line;
    let turns = 2.0;
    let segments = 24;
    let mut prev = (x + size / 2.0, y + size / 2.0);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = turns * std::f32::consts::TAU * t;
        let radius = (size / 2.0) * t;
        let px = x + size / 2.0 + radius * angle.cos();
        let py = y + size / 2.0 + radius * angle.sin();
        draw_line(prev.0, prev.1, px, py, 2.0, color);
        prev = (px, py);
    }
}

pub fn draw_card(card_rect: &CardRect, is_ally: bool) {
    let rect = card_rect.rect;
    draw_texture_ex(
        &card_rect.image,
        rect.x,
        rect.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(rect.w, rect.h)),
            rotation: card_rect.rotation(),
            ..Default::default()
        },
    );

    let mut sleeve_color = DARKGREEN;
    if !is_ally {
        sleeve_color = RED;
    }

    // Draw rectangle border rotated around the center
    let w = rect.w;
    let h = rect.h;
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

    if card_rect.modifiers.contains(&Modifier::SummoningSickness) {
        let icon_size = 22.0;
        let scale = 1.0;
        let x = card_rect.rect.x + card_rect.rect.w * scale - icon_size - 4.0;
        let y = card_rect.rect.y + 4.0;
        draw_vortex_icon(x, y, icon_size, BLUE);
    }

    if card_rect.modifiers.contains(&Modifier::Disabled) {
        let icon_size = 15.0;
        let x = card_rect.rect.x + card_rect.rect.w - 30.0 - 5.0;
        let y = card_rect.rect.y + 4.0;
        let cx = x + icon_size / 2.0;
        let cy = y + icon_size / 2.0;
        draw_circle_lines(cx, cy, icon_size / 2.0, 3.0, WHITE);
        draw_line(x + 4.0, y + icon_size - 4.0, x + icon_size - 4.0, y + 4.0, 3.0, WHITE);
    }

    // Draw damage taken indicator if damage_taken > 0
    if card_rect.damage_taken > 0 {
        let circle_radius = 8.0;
        let circle_x = rect.x + w - circle_radius - 3.0;
        let circle_y = rect.y + circle_radius - 3.0;
        draw_circle(
            circle_x + circle_radius,
            circle_y + circle_radius,
            circle_radius - 2.0,
            RED,
        );
        let dmg_text = card_rect.damage_taken.to_string();
        let text_dims = macroquad::text::measure_text(&dmg_text, None, 12, 1.0);
        draw_text(
            &dmg_text,
            circle_x + circle_radius - text_dims.width / 2.0,
            circle_y + circle_radius + text_dims.height / 2.8,
            12.0,
            WHITE,
        );
    }
}

pub async fn render_card_preview(card: &CardRect, data: &mut GameData) -> anyhow::Result<()> {
    if let Status::SelectingCard { preview: true, .. } = &data.status {
        return Ok(());
    }

    let screen_rect = crate::config::screen_rect();
    let mut rect = card.rect;
    let mut preview_scale: f32 = realm_rect().x / card.rect.w;
    if rect.w > rect.h {
        preview_scale = realm_rect().x / card.rect.h;
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
