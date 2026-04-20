use crate::{
    components::{Component, ComponentCommand, ComponentType},
    config::CARD_ASPECT_RATIO,
    render::{self},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};
use sorcerers::{
    card::{CardData, Zone},
    networking::{client::Client, message::ClientMessage},
};

const THUMB_W: f32 = 80.0;
const THUMB_H: f32 = THUMB_W / CARD_ASPECT_RATIO;
const CARD_PAD: f32 = 10.0;
/// Height of the visible strip exposed by each backing card in a stack.
const STACK_STRIP: f32 = 22.0;
/// Maximum cards per column before a new column is started.
const MAX_PER_COLUMN: usize = 10;

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
        .collect::<Vec<&CardData>>();

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

                            let col_h = STACK_STRIP * n.saturating_sub(1) as f32 + THUMB_H;
                            let (col_rect, _) =
                                ui.allocate_exact_size(vec2(THUMB_W, col_h), Sense::hover());

                            if !ui.is_rect_visible(col_rect) {
                                continue;
                            }

                            // Draw all card images / placeholders back-to-front.
                            // Each successive card covers the lower portion of the one
                            // before it, leaving only the STACK_STRIP strip visible.
                            for local_i in 0..n {
                                let gi = start + local_i;
                                let y = col_rect.min.y + local_i as f32 * STACK_STRIP;
                                let card_rect = Rect::from_min_size(
                                    pos2(col_rect.min.x, y),
                                    vec2(THUMB_W, THUMB_H),
                                );

                                let texture =
                                    TextureCache::get_card_texture_blocking(cards[gi], ui.ctx());
                                if let Some(ref tex) = texture {
                                    let resp =
                                        ui.allocate_rect(card_rect, Sense::CLICK | Sense::HOVER);
                                    ui.painter().image(
                                        tex.id(),
                                        card_rect,
                                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );

                                    if resp.clicked() {
                                        handle_card_click(
                                            data, cards[gi], client, game_id, player_id,
                                        )
                                        .expect("handling card click failed");
                                    }

                                    // Draw border after the image so it appears on top. Highlight
                                    // if hovered.
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
                                    let painter = ui.painter();
                                    painter.rect_filled(card_rect, 4.0, Color32::DARK_GRAY);
                                    // For the front card with no image, show name in centre.
                                    if local_i == n - 1 {
                                        painter.text(
                                            card_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            cards[gi].get_name(),
                                            egui::FontId::proportional(9.0),
                                            Color32::LIGHT_GRAY,
                                        );
                                    }
                                }
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
        } = command
        {
            // If a viewer for this zone+controller already exists, toggle its visibility.
            if let Some(entry) = self
                .viewers
                .iter_mut()
                .find(|e| &e.zone == zone && &e.controller_id == controller_id)
            {
                entry.visible = !entry.visible;
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
