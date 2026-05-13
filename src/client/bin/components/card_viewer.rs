use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    render::{self},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};
use sorcerers::{
    card::CardData,
    networking::{client::Client, message::ClientMessage},
    zone::Zone,
};

const THUMB_W: f32 = 80.0;
const THUMB_H: f32 = THUMB_W / CARD_ASPECT_RATIO;
const CARD_PAD: f32 = 10.0;
/// Height of the visible strip exposed by each backing card in a stack.
const STACK_STRIP: f32 = 22.0;
/// Maximum cards per column before a new column is started.
const MAX_PER_COLUMN: usize = 10;

fn thumb_size(card: &CardData) -> egui::Vec2 {
    if card.is_site() {
        vec2(THUMB_H, THUMB_W)
    } else {
        vec2(THUMB_W, THUMB_H)
    }
}

fn draw_card_thumb(
    data: &GameData,
    card: &CardData,
    card_rect: Rect,
    ui: &mut Ui,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &uuid::Uuid,
) {
    let texture = TextureCache::get_card_texture_blocking(card, ui.ctx());
    if let Some(ref tex) = texture {
        let resp = ui.allocate_rect(card_rect, Sense::CLICK | Sense::HOVER);
        ui.painter().image(
            tex.id(),
            card_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        if resp.clicked() {
            handle_card_click(data, card, client, game_id, player_id)
                .expect("handling card click failed");
        }

        let border_color = if resp.hovered() {
            render::draw_card_preview(ui, Some(tex)).unwrap();
            Color32::WHITE
        } else {
            Color32::from_gray(100)
        };

        ui.painter().rect_stroke(
            card_rect,
            4.0,
            Stroke::new(1.0, border_color),
            egui::StrokeKind::Outside,
        );
    } else {
        let resp = ui.allocate_rect(card_rect, Sense::CLICK | Sense::HOVER);
        if resp.clicked() {
            handle_card_click(data, card, client, game_id, player_id)
                .expect("handling card click failed");
        }

        let painter = ui.painter();
        painter.rect_filled(card_rect, 4.0, Color32::DARK_GRAY);
        painter.text(
            card_rect.center(),
            egui::Align2::CENTER_CENTER,
            card.get_name(),
            egui::FontId::proportional(9.0),
            Color32::LIGHT_GRAY,
        );
    }
}

fn render_hand_viewer(
    cards: &[CardData],
    data: &mut GameData,
    ui: &mut Ui,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &uuid::Uuid,
) {
    let spells = cards
        .iter()
        .filter(|card| card.is_spell())
        .collect::<Vec<_>>();
    let sites = cards
        .iter()
        .filter(|card| card.is_site())
        .collect::<Vec<_>>();

    let spell_dim = vec2(THUMB_W, THUMB_H);
    let site_dim = vec2(THUMB_H, THUMB_W);
    let min_visible_width = spell_dim.x * 0.25;
    let available_w = ui.available_width().max(520.0) - CARD_PAD * 2.0;
    let spell_spacing = if spells.len() > 1 {
        ((available_w - spell_dim.x) / (spells.len() as f32 - 1.0))
            .min(spell_dim.x - min_visible_width)
            .max(10.0)
    } else {
        0.0
    };
    let spells_width = if spells.is_empty() {
        0.0
    } else {
        spell_dim.x + (spells.len() as f32 - 1.0) * spell_spacing
    };

    let sites_per_column = 4;
    let site_columns = sites.len().div_ceil(sites_per_column);
    let site_spacing_y = (site_dim.y * 0.15).max(20.0);
    let site_spacing_x = 20.0;
    let sites_width = if sites.is_empty() {
        0.0
    } else {
        site_columns as f32 * site_dim.x + site_columns.saturating_sub(1) as f32 * site_spacing_x
    };
    let total_width = spells_width
        + if sites.is_empty() {
            0.0
        } else {
            site_spacing_x + sites_width
        };
    let sites_height = if sites.is_empty() {
        0.0
    } else {
        let rows = sites.len().min(sites_per_column);
        site_dim.y + rows.saturating_sub(1) as f32 * site_spacing_y
    };
    let content_size = vec2(
        total_width.max(available_w).max(THUMB_W),
        spell_dim.y.max(sites_height).max(THUMB_H),
    );

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content_rect, _) = ui.allocate_exact_size(content_size, Sense::hover());
            let start_x = content_rect.min.x + (content_rect.width() - total_width) / 2.0;
            let spells_y = content_rect.center().y - spell_dim.y / 2.0;

            for (idx, card) in spells.iter().enumerate() {
                let x = start_x + idx as f32 * spell_spacing;
                let rect = Rect::from_min_size(pos2(x, spells_y), spell_dim);
                draw_card_thumb(data, card, rect, ui, client, game_id, player_id);
            }

            if !sites.is_empty() {
                let sites_x = start_x + spells_width + site_spacing_x;
                let sites_y = content_rect.center().y - spell_dim.y / 2.0;
                for (idx, card) in sites.iter().enumerate() {
                    let col = idx / sites_per_column;
                    let row = idx % sites_per_column;
                    let x = sites_x + col as f32 * (site_dim.x + site_spacing_x);
                    let y = sites_y + row as f32 * site_spacing_y;
                    let rect = Rect::from_min_size(pos2(x, y), site_dim);
                    draw_card_thumb(data, card, rect, ui, client, game_id, player_id);
                }
            }
        });
}

#[derive(Debug)]
struct ViewerEntry {
    title: String,
    zone: Zone,
    controller_id: Option<uuid::Uuid>,
    visible: bool,
}

#[derive(Debug)]
pub struct CardViewerComponent {
    viewers: Vec<ViewerEntry>,
    client: Client,
    game_id: uuid::Uuid,
    player_id: uuid::Uuid,
}

impl CardViewerComponent {
    pub fn new(game_id: &uuid::Uuid, player_id: &uuid::Uuid, client: Client) -> Self {
        Self {
            viewers: Vec::new(),
            client,
            game_id: *game_id,
            player_id: *player_id,
        }
    }
}

fn handle_card_click(
    data: &GameData,
    card: &CardData,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &uuid::Uuid,
) -> anyhow::Result<()> {
    match &data.status {
        Status::Idle => {
            client.send(ClientMessage::ClickCard {
                game_id: *game_id,
                player_id: *player_id,
                card_id: card.id,
            })?;

            Ok(())
        }
        Status::SelectingCard {
            cards,
            multiple: false,
            ..
        } => {
            if !cards.contains(&card.id) {
                return Ok(());
            }

            client.send(ClientMessage::PickCard {
                game_id: *game_id,
                player_id: *player_id,
                card_id: card.id,
            })?;

            Ok(())
        }
        _ => Ok(()),
    }
}

fn render_viewer(
    entry: &mut ViewerEntry,
    data: &mut GameData,
    ui: &mut Ui,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &uuid::Uuid,
) {
    let cards = data
        .cards
        .iter()
        .filter(|c| c.zone == entry.zone && entry.controller_id.is_none_or(|id| c.owner_id == id))
        .cloned()
        .collect::<Vec<CardData>>();

    let window_id = egui::Id::new(format!("{:?}-{:?}", entry.zone, entry.controller_id));
    let mut open = entry.visible;
    egui::Window::new(entry.title.clone())
        .id(window_id)
        .open(&mut open)
        .movable(true)
        .resizable(true)
        .min_width(THUMB_W + CARD_PAD * 2.0)
        .min_height(THUMB_H + CARD_PAD * 2.0)
        .default_size(vec2(600.0, 400.0))
        .show(ui.ctx(), |ui| {
            if cards.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("(empty)")
                            .color(Color32::GRAY)
                            .size(14.0),
                    );
                });
                return;
            }

            if entry.zone == Zone::Hand {
                render_hand_viewer(&cards, data, ui, client, game_id, player_id);
                return;
            }

            let total = cards.len();
            let num_cols = total.div_ceil(MAX_PER_COLUMN);
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing = vec2(CARD_PAD, 0.0);

                        for col_idx in 0..num_cols {
                            let start = col_idx * MAX_PER_COLUMN;
                            let end = (start + MAX_PER_COLUMN).min(total);
                            let n = end - start;
                            let col_w = cards[start..end]
                                .iter()
                                .map(|card| thumb_size(card).x)
                                .fold(THUMB_W, f32::max);
                            let col_h = cards[start..end]
                                .iter()
                                .enumerate()
                                .map(|(idx, card)| idx as f32 * STACK_STRIP + thumb_size(card).y)
                                .fold(0.0, f32::max);

                            let (col_rect, _) =
                                ui.allocate_exact_size(vec2(col_w, col_h), Sense::hover());

                            if !ui.is_rect_visible(col_rect) {
                                continue;
                            }

                            // Draw all card images / placeholders back-to-front.
                            // Each successive card covers the lower portion of the one
                            // before it, leaving only the STACK_STRIP strip visible.
                            for local_i in 0..n {
                                let gi = start + local_i;
                                let y = col_rect.min.y + local_i as f32 * STACK_STRIP;
                                let size = thumb_size(&cards[gi]);
                                let x = col_rect.min.x + (col_w - size.x) / 2.0;
                                let card_rect = Rect::from_min_size(pos2(x, y), size);

                                draw_card_thumb(
                                    data, &cards[gi], card_rect, ui, client, game_id, player_id,
                                );
                            }
                        }
                    });
                });
        });

    entry.visible = open;
}

impl Component for CardViewerComponent {
    fn update(&mut self, _data: &mut GameData, _ctx: &Context) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(
        &mut self,
        data: &mut GameData,
        ui: &mut Ui,
        _painter: &Painter,
    ) -> anyhow::Result<Option<ComponentCommand>> {
        let client = self.client.clone();
        let game_id = self.game_id;
        let player_id = self.player_id;

        for entry in &mut self.viewers {
            if !entry.visible {
                continue;
            }
            render_viewer(entry, data, ui, &client, &game_id, &player_id);
        }

        Ok(None)
    }

    fn process_command(
        &mut self,
        command: &ComponentCommand,
        _data: &mut GameData,
    ) -> anyhow::Result<()> {
        if let ComponentCommand::OpenCardViewer {
            title,
            zone,
            controller_id,
            open_only,
        } = command
        {
            // If a viewer for this zone+controller already exists, either show it or toggle it.
            if let Some(entry) = self
                .viewers
                .iter_mut()
                .find(|e| &e.zone == zone && &e.controller_id == controller_id)
            {
                entry.visible = if *open_only { true } else { !entry.visible };
                // Update title in case the caller changed it.
                entry.title = title.clone();
            } else {
                self.viewers.push(ViewerEntry {
                    title: title.clone(),
                    zone: zone.clone(),
                    controller_id: *controller_id,
                    visible: true,
                });
            }
        }
        Ok(())
    }

    fn toggle_visibility(&mut self) {
        let any_visible = self.viewers.iter().any(|e| e.visible);
        for entry in &mut self.viewers {
            entry.visible = !any_visible;
        }
    }

    fn is_visible(&self) -> bool {
        self.viewers.iter().any(|e| e.visible)
    }

    fn get_component_type(&self) -> ComponentType {
        ComponentType::CardViewer
    }
}
