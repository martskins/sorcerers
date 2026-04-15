use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::{card_height, card_width, screen_rect},
    input::Mouse,
    render::{self, CardRect},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Ui, pos2, vec2};
use sorcerers::{
    card::{CardData, Zone},
    game::PlayerId,
    networking::{self, message::ClientMessage},
};
use std::collections::HashSet;

const FONT_SIZE: f32 = 24.0;
/// Height reserved for a zone-group header label row.
const HEADER_H: f32 = 28.0;
/// Gap between the header label and the first card row.
const HEADER_GAP: f32 = 8.0;
/// Vertical gap between consecutive zone groups.
const GROUP_GAP: f32 = 20.0;
/// Font size for zone-group header labels.
const HEADER_FONT: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionOverlayBehaviour {
    Preview,
    Pick,
}

#[derive(Debug)]
pub struct SelectionOverlay {
    card_rects: Vec<CardRect>,
    /// Zone group headers: (label text, centre position on screen).
    zone_group_headers: Vec<(String, egui::Pos2)>,
    prompt: String,
    behaviour: SelectionOverlayBehaviour,
    close: bool,
    player_id: PlayerId,
    game_id: uuid::Uuid,
    client: networking::client::Client,
    pickable_cards: HashSet<uuid::Uuid>,
}

/// Maps a card's zone + ownership into a (sort_order, display_label) pair.
fn zone_order_and_label(zone: &Zone, is_mine: bool) -> (u32, String) {
    match zone {
        Zone::Hand => (
            if is_mine { 0 } else { 1 },
            if is_mine {
                "Your Hand".into()
            } else {
                "Opponent's Hand".into()
            },
        ),
        Zone::Realm(_) | Zone::Intersection(_) => (2, "In Play".into()),
        Zone::Cemetery => (
            if is_mine { 3 } else { 4 },
            if is_mine {
                "Your Cemetery".into()
            } else {
                "Opponent's Cemetery".into()
            },
        ),
        Zone::Spellbook => (
            if is_mine { 5 } else { 6 },
            if is_mine {
                "Your Spellbook".into()
            } else {
                "Opponent's Spellbook".into()
            },
        ),
        Zone::Atlasbook => (
            if is_mine { 7 } else { 8 },
            if is_mine {
                "Your Atlas".into()
            } else {
                "Opponent's Atlas".into()
            },
        ),
        Zone::Banish => (9, "Banished".into()),
        _ => (10, "Other".into()),
    }
}

/// Groups `cards` by zone (and ownership) in a canonical display order.
fn group_cards_by_zone<'a>(
    cards: &[&'a CardData],
    player_id: &PlayerId,
) -> Vec<(String, Vec<&'a CardData>)> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<u32, (String, Vec<&'a CardData>)> = BTreeMap::new();
    for &card in cards {
        let is_mine = &card.owner_id == player_id;
        let (order, label) = zone_order_and_label(&card.zone, is_mine);
        map.entry(order)
            .or_insert_with(|| (label, Vec::new()))
            .1
            .push(card);
    }
    map.into_values().collect()
}

impl SelectionOverlay {
    pub fn new(
        client: networking::client::Client,
        game_id: &uuid::Uuid,
        player_id: &PlayerId,
        cards: Vec<&CardData>,
        pickable_cards: Vec<uuid::Uuid>,
        prompt: &str,
        behaviour: SelectionOverlayBehaviour,
    ) -> Self {
        let (card_rects, zone_group_headers) = match behaviour {
            SelectionOverlayBehaviour::Preview => {
                (Self::build_preview_rects(&cards), vec![])
            }
            SelectionOverlayBehaviour::Pick => {
                Self::build_pick_rects_grouped(&cards, player_id)
            }
        };

        let pickable_cards = if pickable_cards.is_empty() {
            cards.iter().map(|card| card.id).collect()
        } else {
            pickable_cards.into_iter().collect()
        };

        Self {
            client,
            game_id: game_id.clone(),
            card_rects,
            zone_group_headers,
            prompt: prompt.to_string(),
            behaviour,
            player_id: player_id.clone(),
            close: false,
            pickable_cards,
        }
    }

    fn build_preview_rects(cards: &[&CardData]) -> Vec<CardRect> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        if cards.is_empty() {
            return Vec::new();
        }
        let card_spacing = 20.0;
        let card_count = cards.len();
        let cw = card_width().unwrap_or(80.0) * 2.0;
        let ch = card_height().unwrap_or(112.0) * 2.0;
        let cards_area_width = card_count as f32 * cw + (card_count as f32 - 1.0) * card_spacing;
        let cards_start_x = (sw - cards_area_width) / 2.0;
        let cards_y = (sh - ch) / 2.0 + 30.0;

        let mut rects = Vec::with_capacity(cards.len());
        for (idx, card) in cards.iter().enumerate() {
            let mut size = vec2(cw, ch);
            if card.is_site() {
                std::mem::swap(&mut size.x, &mut size.y);
            }
            let x = cards_start_x + idx as f32 * (size.x + card_spacing);
            rects.push(CardRect {
                image: None,
                rect: Rect::from_min_size(pos2(x, cards_y), size),
                is_hovered: false,
                is_selected: false,
                card: (*card).clone(),
            });
        }

        rects
    }

    /// Lays out cards grouped by zone, stacked vertically on screen.
    /// Returns both the card rects and the zone-group header positions.
    fn build_pick_rects_grouped(
        cards: &[&CardData],
        player_id: &PlayerId,
    ) -> (Vec<CardRect>, Vec<(String, egui::Pos2)>) {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);

        let base_w = card_width().unwrap_or(80.0);
        let base_h = card_height().unwrap_or(112.0);
        let row_step = (base_h * 0.18).max(18.0);
        let card_gap_x = 22.0;
        let column_w = base_w.max(base_h);

        let groups = group_cards_by_zone(cards, player_id);
        if groups.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let top_margin = 70.0; // room for the prompt text
        let bottom_margin = 80.0;

        // Distribute available vertical space evenly across groups.
        let header_overhead = groups.len() as f32 * (HEADER_H + HEADER_GAP)
            + (groups.len().saturating_sub(1)) as f32 * GROUP_GAP;
        let available_h =
            (sh - top_margin - bottom_margin - header_overhead).max(base_h);
        let per_group_h = available_h / groups.len() as f32;
        let cards_per_col = (((per_group_h - base_h) / row_step).floor() as usize + 1)
            .clamp(1, 10);

        let mut all_rects: Vec<CardRect> = Vec::new();
        let mut headers: Vec<(String, egui::Pos2)> = Vec::new();
        let mut current_y = top_margin;

        for (label, group_cards) in &groups {
            let n = group_cards.len();
            let num_cols = n.div_ceil(cards_per_col);
            // Balance cards across columns.
            let actual_per_col = n.div_ceil(num_cols);
            let col_h =
                row_step * actual_per_col.saturating_sub(1) as f32 + base_h;

            let total_w = num_cols as f32 * column_w
                + (num_cols.saturating_sub(1)) as f32 * card_gap_x;
            let start_x = (sw - total_w) / 2.0;

            // Header centred horizontally.
            headers.push((
                label.clone(),
                egui::pos2(sw / 2.0, current_y + HEADER_H / 2.0),
            ));

            let cards_y = current_y + HEADER_H + HEADER_GAP;

            for (ci, chunk) in group_cards.chunks(actual_per_col).enumerate() {
                let col_x = start_x + ci as f32 * (column_w + card_gap_x);
                for (ri, card) in chunk.iter().enumerate() {
                    let size = if card.is_site() {
                        vec2(base_h, base_w)
                    } else {
                        vec2(base_w, base_h)
                    };
                    let x = col_x + (column_w - size.x) / 2.0;
                    let y = cards_y + ri as f32 * row_step;
                    all_rects.push(CardRect {
                        image: None,
                        rect: Rect::from_min_size(pos2(x, y), size),
                        is_hovered: false,
                        is_selected: false,
                        card: (*card).clone(),
                    });
                }
            }

            current_y = cards_y + col_h + GROUP_GAP;
        }

        (all_rects, headers)
    }

    fn hovered_card_index(&self, ctx: &Context) -> Option<usize> {
        let mouse_pos = Mouse::position(ctx)?;
        let mut hovered = None;
        for (idx, card_rect) in self.card_rects.iter().enumerate() {
            if card_rect.rect.contains(mouse_pos) {
                hovered = Some(idx);
            }
        }
        hovered
    }

    fn update_hover_state(&mut self, ctx: &Context) {
        let hovered = self.hovered_card_index(ctx);
        for card_rect in &mut self.card_rects {
            card_rect.is_hovered = false;
        }
        if let Some(idx) = hovered {
            if let Some(card_rect) = self.card_rects.get_mut(idx) {
                card_rect.is_hovered = true;
            }
        }
    }

    fn is_pickable(&self, card_id: &uuid::Uuid) -> bool {
        self.pickable_cards.contains(card_id)
    }

    fn hovered_card(&self) -> Option<&CardRect> {
        self.card_rects.iter().rev().find(|card| card.is_hovered)
    }

    fn render_hover_preview(&self, hovered: &CardRect, ui: &Ui, painter: &Painter) {
        let screen =
            screen_rect().unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0)));
        let mouse = Mouse::position(ui.ctx()).unwrap_or(hovered.rect.center());
        let preview_scale = 2.0;
        let preview_size = vec2(
            hovered.rect.width() * preview_scale,
            hovered.rect.height() * preview_scale,
        );
        let mut preview_pos = mouse + vec2(28.0, 28.0);
        if preview_pos.x + preview_size.x > screen.max.x - 8.0 {
            preview_pos.x = mouse.x - preview_size.x - 28.0;
        }
        if preview_pos.y + preview_size.y > screen.max.y - 8.0 {
            preview_pos.y = screen.max.y - preview_size.y - 8.0;
        }
        let min_x = screen.min.x + 8.0;
        let min_y = screen.min.y + 8.0;
        let max_x = (screen.max.x - preview_size.x - 8.0).max(min_x);
        let max_y = (screen.max.y - preview_size.y - 8.0).max(min_y);
        preview_pos.x = preview_pos.x.clamp(min_x, max_x);
        preview_pos.y = preview_pos.y.clamp(min_y, max_y);

        let preview_rect = Rect::from_min_size(preview_pos, preview_size);
        let preview_card = CardRect {
            rect: preview_rect,
            image: hovered.image.clone(),
            is_hovered: false,
            is_selected: false,
            card: hovered.card.clone(),
        };

        painter.rect_filled(
            preview_rect.expand(6.0),
            8.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 140),
        );
        render::draw_card(&preview_card, true, false, painter);
    }
}

impl Component for SelectionOverlay {
    fn update(&mut self, _data: &mut GameData, ctx: &Context) -> anyhow::Result<()> {
        for card_rect in &mut self.card_rects {
            if card_rect.image.is_none() {
                card_rect.image = TextureCache::get_card_texture_blocking(&card_rect.card, ctx);
            }
        }
        self.update_hover_state(ctx);
        Ok(())
    }

    fn process_command(
        &mut self,
        _command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn toggle_visibility(&mut self) {}

    fn is_visible(&self) -> bool {
        !self.close
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::SelectionOverlay
    }

    fn process_input(
        &mut self,
        _in_turn: bool,
        data: &mut GameData,
        ctx: &Context,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        if !Mouse::enabled() {
            return Ok(None);
        }

        self.update_hover_state(ctx);
        let clicked = Mouse::clicked(ctx);

        if !clicked {
            return Ok(None);
        }

        let hovered = match self.hovered_card() {
            Some(card) => card,
            None => return Ok(None),
        };

        match self.behaviour {
            SelectionOverlayBehaviour::Preview => {
                self.client.send(ClientMessage::ClickCard {
                    game_id: self.game_id,
                    player_id: self.player_id,
                    card_id: hovered.card.id,
                })?;
                self.close = true;
            }
            SelectionOverlayBehaviour::Pick => {
                if self.is_pickable(&hovered.card.id) {
                    self.client.send(ClientMessage::PickCard {
                        game_id: self.game_id,
                        player_id: self.player_id,
                        card_id: hovered.card.id,
                    })?;
                    self.close = true;
                }
            }
        }

        data.status = Status::Idle;

        Ok(None)
    }

    fn render(
        &mut self,
        _data: &mut GameData,
        ui: &mut Ui,
        painter: &Painter,
    ) -> anyhow::Result<()> {
        let sw = screen_rect().map(|r| r.width()).unwrap_or(1280.0);
        let sh = screen_rect().map(|r| r.height()).unwrap_or(720.0);
        let title = ui.fonts(|f| {
            f.layout_no_wrap(
                self.prompt.clone(),
                egui::FontId::proportional(FONT_SIZE),
                Color32::WHITE,
            )
        });

        painter.rect_filled(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(sw, sh)),
            0.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 204),
        );
        painter.galley(
            pos2(sw / 2.0 - title.size().x / 2.0, 30.0),
            title,
            Color32::WHITE,
        );

        for card_rect in &self.card_rects {
            render::draw_card(
                card_rect,
                card_rect.card.controller_id == self.player_id,
                false,
                painter,
            );
            if self.behaviour == SelectionOverlayBehaviour::Pick
                && !self.is_pickable(&card_rect.card.id)
            {
                painter.rect_filled(
                    card_rect.rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                );
            }
        }

        // Draw zone-group headers above each group's cards.
        for (label, centre) in &self.zone_group_headers {
            let galley = ui.fonts(|f| {
                f.layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(HEADER_FONT),
                    Color32::WHITE,
                )
            });
            let text_size = galley.size();
            let bg_rect = Rect::from_center_size(*centre, text_size + vec2(20.0, 8.0));
            painter.rect_filled(
                bg_rect,
                4.0,
                Color32::from_rgba_unmultiplied(20, 20, 70, 220),
            );
            // Horizontal separator lines extending to the left and right of the pill.
            let sep_y = centre.y;
            let sep_pad = 12.0;
            painter.line_segment(
                [pos2(0.0, sep_y), pos2(bg_rect.min.x - sep_pad, sep_y)],
                egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 120, 200, 100)),
            );
            painter.line_segment(
                [pos2(bg_rect.max.x + sep_pad, sep_y), pos2(sw, sep_y)],
                egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 120, 200, 100)),
            );
            painter.galley(
                pos2(centre.x - text_size.x / 2.0, centre.y - text_size.y / 2.0),
                galley,
                Color32::WHITE,
            );
        }

        if let Some(hovered) = self.hovered_card() {
            if self.behaviour == SelectionOverlayBehaviour::Pick {
                self.render_hover_preview(hovered, ui, painter);
            }
        }

        if self.behaviour == SelectionOverlayBehaviour::Preview {
            let close_button_pos = pos2(sw / 2.0 - 50.0, sh - 70.0);
            egui::Area::new(egui::Id::new("selection_close_btn"))
                .fixed_pos(close_button_pos)
                .show(ui.ctx(), |ui| {
                    let close = egui::Button::new(
                        egui::RichText::new("Close")
                            .size(22.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(vec2(120.0, 44.0));
                    if ui.add(close).clicked() {
                        self.close = true;
                    }
                });
        }

        Ok(())
    }
}
