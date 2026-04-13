/// Shared utilities for drawing elemental threshold icons (triangle glyphs).
///
/// Fire  = upward triangle              (▲)  red
/// Air   = upward triangle + midline         purple
/// Earth = downward triangle + midline       brown
/// Water = downward triangle            (▽)  blue
use egui::{Color32, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};
use sorcerers::game::{Element, Thresholds};

// ── Colours ──────────────────────────────────────────────────────────────────

pub fn element_color(element: &Element) -> Color32 {
    match element {
        Element::Fire => Color32::from_rgb(220, 70, 40),
        Element::Air => Color32::from_rgb(160, 90, 220),
        Element::Earth => Color32::from_rgb(140, 100, 40),
        Element::Water => Color32::from_rgb(50, 150, 230),
    }
}

// ── Core drawing primitive ───────────────────────────────────────────────────

/// Draw a single element triangle centred at `center` with the given `size`.
/// The outline stroke width is `stroke_width`.
pub fn draw_element_triangle(
    painter: &Painter,
    element: &Element,
    center: Pos2,
    size: f32,
    stroke_width: f32,
) {
    let col = element_color(element);
    let hs = size / 2.0;

    let is_upward = matches!(element, Element::Fire | Element::Air);
    let has_midline = matches!(element, Element::Air | Element::Earth);

    let (v1, v2, v3) = if is_upward {
        (
            pos2(center.x, center.y - hs),      // top apex
            pos2(center.x - hs, center.y + hs), // bottom-left
            pos2(center.x + hs, center.y + hs), // bottom-right
        )
    } else {
        (
            pos2(center.x - hs, center.y - hs), // top-left
            pos2(center.x + hs, center.y - hs), // top-right
            pos2(center.x, center.y + hs),      // bottom apex
        )
    };

    painter.add(Shape::closed_line(
        vec![v1, v2, v3],
        Stroke::new(stroke_width, col),
    ));

    if has_midline {
        painter.line_segment(
            [pos2(center.x - hs, center.y), pos2(center.x + hs, center.y)],
            Stroke::new(stroke_width, col),
        );
    }
}

// ── Higher-level helpers ─────────────────────────────────────────────────────

/// Draw a row of element threshold symbols into a `&Painter` starting at `(x, y)`.
/// Returns the new `x` position after all symbols.
/// `size` is the triangle bounding-box side length.
pub fn draw_thresholds(
    painter: &Painter,
    x: f32,
    y: f32,
    thresholds: &Thresholds,
    size: f32,
    stroke_width: f32,
) -> f32 {
    let mut cx = x;
    for (count, element) in [
        (thresholds.fire, Element::Fire),
        (thresholds.air, Element::Air),
        (thresholds.earth, Element::Earth),
        (thresholds.water, Element::Water),
    ] {
        if count == 0 {
            continue;
        }

        for _ in 0..count {
            let center = pos2(cx + size / 2.0, y + size / 2.0);
            draw_element_triangle(painter, &element, center, size, stroke_width);
            cx += size + 3.0;
        }
    }
    cx
}

/// Draw a single element symbol as an egui widget (allocates space in `ui`).
/// Suitable for inline use inside `ui.horizontal`.
/// Renders `count` as a label beside the triangle.
pub fn element_symbol_widget(
    ui: &mut Ui,
    count: u8,
    element: &Element,
    size: f32,
    stroke_width: f32,
) {
    let col = element_color(element);
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let center = rect.center();
    draw_element_triangle(ui.painter(), element, center, size, stroke_width);
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(count.to_string())
            .color(col)
            .size(size * 0.85)
            .strong(),
    );
    ui.add_space(6.0);
}

/// Draw a small element icon button for filter UIs (allocates space via `ui.allocate_painter`).
/// Returns `true` if clicked.
pub fn element_filter_button(
    ui: &mut Ui,
    element: &Element,
    active: bool,
    btn_size: Vec2,
    icon_size: f32,
    stroke_width: f32,
) -> bool {
    let col = element_color(element);
    let active_bg = Color32::from_rgb(50, 70, 120);
    let idle_bg = Color32::from_rgb(25, 30, 55);

    let (resp, painter) = ui.allocate_painter(btn_size, Sense::click());
    let r = resp.rect;

    painter.rect_filled(
        r,
        egui::CornerRadius::same(3),
        if active { active_bg } else { idle_bg },
    );
    if active {
        painter.rect_stroke(
            r,
            egui::CornerRadius::same(3),
            Stroke::new(1.0, col),
            egui::StrokeKind::Outside,
        );
    }

    draw_element_triangle(&painter, element, r.center(), icon_size, stroke_width);
    resp.clicked()
}
