use crate::{
    card_layout::{self, CardDims, HandLayout},
    components::{Component, ComponentCommand, ComponentType},
    render::{self},
    scene::game::{GameData, Status},
    texture_cache::TextureCache,
};
use egui::{Color32, Context, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};
use sorcerers::{
    card::CardData,
    game::PlayerId,
    networking::{client::Client, message::ClientMessage},
    zone::Zone,
};

fn thumb_size(card: &CardData) -> egui::Vec2 {
    CardDims::from_spell_width(card_layout::DEFAULT_THUMB_W).for_card(card)
}

fn draw_card_thumb(
    data: &mut GameData,
    card: &CardData,
    card_rect: Rect,
    ui: &mut Ui,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &PlayerId,
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

        if resp.clicked()
            && let Err(e) = handle_card_click(data, card, client, game_id, player_id)
        {
            eprintln!("Error handling card click: {}", e);
        }

        let border_color = if resp.hovered() {
            if let Err(e) = render::draw_card_preview(ui, Some(tex)) {
                eprintln!("Error drawing card preview: {}", e);
            }
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
        if matches!(
            &data.status,
            Status::SelectingCard {
                pickable_cards,
                multiple: false,
                ..
            } if !pickable_cards.contains(&card.id)
        ) {
            ui.painter().rect_filled(
                card_rect,
                4.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 110),
            );
        }
    } else {
        let resp = ui.allocate_rect(card_rect, Sense::CLICK | Sense::HOVER);
        if resp.clicked()
            && let Err(e) = handle_card_click(data, card, client, game_id, player_id)
        {
            eprintln!("Error handling card click: {}", e);
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
    player_id: &PlayerId,
) {
    let spells = cards
        .iter()
        .filter(|card| card.is_spell())
        .collect::<Vec<_>>();
    let sites = cards
        .iter()
        .filter(|card| card.is_site())
        .collect::<Vec<_>>();

    let dims = CardDims::from_spell_width(card_layout::DEFAULT_THUMB_W);
    let available_w = ui.available_width().max(520.0) - card_layout::CARD_PAD * 2.0;
    let layout = HandLayout::new(spells.len(), sites.len(), dims, available_w);
    let content_size = card_layout::hand_content_size(&layout, dims, available_w);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content_rect, _) = ui.allocate_exact_size(content_size, Sense::hover());

            for (idx, card) in spells.iter().enumerate() {
                let rect = card_layout::spell_rect(content_rect, &layout, dims, idx);
                draw_card_thumb(data, card, rect, ui, client, game_id, player_id);
            }

            if !sites.is_empty() {
                for (idx, card) in sites.iter().enumerate() {
                    let rect = card_layout::site_rect(content_rect, &layout, dims, idx);
                    draw_card_thumb(data, card, rect, ui, client, game_id, player_id);
                }
            }
        });
}

#[derive(Debug)]
struct ViewerEntry {
    title: String,
    zone: Zone,
    controller_id: Option<PlayerId>,
    card_ids: Option<Vec<uuid::Uuid>>,
    visible: bool,
}

#[derive(Debug)]
pub struct CardViewerComponent {
    viewers: Vec<ViewerEntry>,
    client: Client,
    game_id: uuid::Uuid,
    player_id: PlayerId,
}

impl CardViewerComponent {
    pub fn new(game_id: &uuid::Uuid, player_id: &PlayerId, client: Client) -> Self {
        Self {
            viewers: Vec::new(),
            client,
            game_id: *game_id,
            player_id: *player_id,
        }
    }
}

fn handle_card_click(
    data: &mut GameData,
    card: &CardData,
    client: &Client,
    game_id: &uuid::Uuid,
    player_id: &PlayerId,
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
            pickable_cards,
            multiple: false,
            ..
        } => {
            if !pickable_cards.contains(&card.id) {
                return Ok(());
            }

            client.send(ClientMessage::PickCard {
                game_id: *game_id,
                player_id: *player_id,
                card_id: card.id,
            })?;
            data.status = Status::Idle;

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
    player_id: &PlayerId,
) {
    let cards = data
        .cards
        .iter()
        .filter(|c| c.zone == entry.zone && entry.controller_id.is_none_or(|id| c.owner_id == id))
        .filter(|c| {
            entry
                .card_ids
                .as_ref()
                .is_none_or(|card_ids| card_ids.contains(&c.id))
        })
        .cloned()
        .collect::<Vec<CardData>>();

    let window_id = egui::Id::new(format!("{:?}-{:?}", entry.zone, entry.controller_id));
    let mut open = entry.visible;
    egui::Window::new(entry.title.clone())
        .id(window_id)
        .order(egui::Order::Tooltip)
        .open(&mut open)
        .movable(true)
        .resizable(true)
        .min_width(card_layout::DEFAULT_THUMB_W + card_layout::CARD_PAD * 2.0)
        .min_height(
            card_layout::DEFAULT_THUMB_W / crate::config::CARD_ASPECT_RATIO
                + card_layout::CARD_PAD * 2.0,
        )
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
            let num_cols = total.div_ceil(card_layout::STACK_MAX_PER_COLUMN);
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing = vec2(card_layout::CARD_PAD, 0.0);

                        for col_idx in 0..num_cols {
                            let start = col_idx * card_layout::STACK_MAX_PER_COLUMN;
                            let end = (start + card_layout::STACK_MAX_PER_COLUMN).min(total);
                            let n = end - start;
                            let col_w = cards[start..end]
                                .iter()
                                .map(|card| thumb_size(card).x)
                                .fold(card_layout::DEFAULT_THUMB_W, f32::max);
                            let col_h = cards[start..end]
                                .iter()
                                .enumerate()
                                .map(|(idx, card)| {
                                    idx as f32 * card_layout::STACK_STRIP + thumb_size(card).y
                                })
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
                                let y = col_rect.min.y + local_i as f32 * card_layout::STACK_STRIP;
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
            card_ids,
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
                entry.card_ids = card_ids.clone();
            } else {
                self.viewers.push(ViewerEntry {
                    title: title.clone(),
                    zone: zone.clone(),
                    controller_id: *controller_id,
                    card_ids: card_ids.clone(),
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
